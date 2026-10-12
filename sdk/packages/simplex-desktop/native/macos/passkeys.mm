#include <node_api.h>
#import <AppKit/AppKit.h>
#import <AuthenticationServices/AuthenticationServices.h>
#import <Security/Security.h>
#include <memory>
#include <string>

// Only Electron's main process loads this library. It owns the window and identity.
@class SimplexPasskeyOperation;
struct State { bool closing = false; SimplexPasskeyOperation* __strong active = nil; };
struct Completion { std::shared_ptr<State> state; napi_deferred deferred; std::string id; };
struct Result { std::string json; std::string code; std::string message; };

static std::string String(napi_env env, napi_value value) {
  size_t size = 0;
  if (napi_get_value_string_utf8(env, value, nullptr, 0, &size) != napi_ok || size > 8192) return {};
  std::string result(size + 1, '\0');
  napi_get_value_string_utf8(env, value, result.data(), result.size(), &size);
  result.resize(size);
  return result;
}
static napi_value Text(napi_env env, const std::string& value) {
  napi_value result; napi_create_string_utf8(env, value.c_str(), value.size(), &result); return result;
}
static NSString* Encode(NSData* data) {
  if (!data) return nil;
  NSString* value = [data base64EncodedStringWithOptions:0];
  value = [[value stringByReplacingOccurrencesOfString:@"+" withString:@"-"] stringByReplacingOccurrencesOfString:@"/" withString:@"_"];
  return [value stringByReplacingOccurrencesOfString:@"=" withString:@""];
}
static NSData* Decode(id value) {
  if (![value isKindOfClass:[NSString class]] || [value length] > 4096) return nil;
  NSString* text = [[value stringByReplacingOccurrencesOfString:@"-" withString:@"+"] stringByReplacingOccurrencesOfString:@"_" withString:@"/"];
  while (text.length % 4) text = [text stringByAppendingString:@"="];
  return [[NSData alloc] initWithBase64EncodedString:text options:0];
}
static NSString* Entitlement(NSString* key) {
  SecTaskRef task = SecTaskCreateFromSelf(kCFAllocatorDefault);
  if (!task) return nil;
  CFTypeRef value = SecTaskCopyValueForEntitlement(task, (__bridge CFStringRef)key, nullptr);
  CFRelease(task);
  return CFBridgingRelease(value);
}
static NSString* Unavailable(NSString* rp) {
  if (@available(macOS 13.0, *)) {} else return @"Native passkeys require macOS 13 or later.";
  if (![NSThread isMainThread]) return @"Native passkeys must run in the application main process.";
  // AuthenticationServices enforces the app's restricted entitlements and domain association.
  // Capability polls check readiness without synchronously validating the whole bundle seal.
  NSString* identifier = Entitlement(@"com.apple.application-identifier");
  NSArray* domains = (id)Entitlement(@"com.apple.developer.associated-domains");
  NSString* domain = [@"webcredentials:" stringByAppendingString:rp ?: @""];
  if (![identifier isKindOfClass:[NSString class]] ||
      ![domains isKindOfClass:[NSArray class]] ||
      (![domains containsObject:domain] && ![domains containsObject:[domain stringByAppendingString:@"?mode=developer"]]))
    return @"This build needs Apple signing and an Associated Domains profile. Use a password or recovery code.";
  NSString* profile = [[NSBundle mainBundle].bundlePath stringByAppendingPathComponent:@"Contents/embedded.provisionprofile"];
  if (![[NSFileManager defaultManager] fileExistsAtPath:profile])
    return @"The Apple provisioning profile is missing. Use a password or recovery code.";
  return nil;
}

API_AVAILABLE(macos(13.0))
@interface SimplexPasskeyOperation : NSObject <ASAuthorizationControllerDelegate, ASAuthorizationControllerPresentationContextProviding>
@property(nonatomic, strong) ASAuthorizationController* controller;
@property(nonatomic, strong) NSWindow* window;
@property(nonatomic, copy) NSString* operationId;
@property(nonatomic) BOOL finished;
@property(nonatomic) napi_threadsafe_function callback;
@property(nonatomic) Completion* completion;
- (void)finish:(Result*)result;
- (void)cancel;
@end

static void Deliver(napi_env env, napi_value, void* context, void* data) {
  std::unique_ptr<Result> result(static_cast<Result*>(data));
  auto* completion = static_cast<Completion*>(context);
  if (!env || completion->state->closing) return;
  if (@available(macOS 13.0, *)) {
    auto active = completion->state->active;
    if (active && std::string(active.operationId.UTF8String) == completion->id) completion->state->active = nil;
  }
  if (result->code.empty()) napi_resolve_deferred(env, completion->deferred, Text(env, result->json));
  else {
    napi_value error;
    napi_create_error(env, Text(env, result->code), Text(env, result->message), &error);
    napi_reject_deferred(env, completion->deferred, error);
  }
}
static void Finalize(napi_env, void* data, void*) { delete static_cast<Completion*>(data); }

@implementation SimplexPasskeyOperation
- (ASPresentationAnchor)presentationAnchorForAuthorizationController:(ASAuthorizationController*)controller { return self.window; }
- (void)finish:(Result*)result {
  if (self.finished) { delete result; return; }
  self.finished = YES;
  if (self.completion->state->closing) {
    delete result;
    napi_release_threadsafe_function(self.callback, napi_tsfn_abort);
  } else {
    if (napi_call_threadsafe_function(self.callback, result, napi_tsfn_nonblocking) != napi_ok) delete result;
    napi_release_threadsafe_function(self.callback, napi_tsfn_release);
  }
  self.completion = nullptr;
}
- (void)cancel {
  if (self.finished) return;
  // Keep the actor until Apple's completion acknowledges cancellation. JS invalidates
  // access immediately; a new sheet must not race the old sheet's dismissal.
  if (self.completion->state->closing) [self finish:new Result{"", "CANCELLED", "Passkey cancelled. Try again."}];
  [self.controller cancel];
}
- (void)authorizationController:(ASAuthorizationController*)controller didCompleteWithError:(NSError*)error {
  const bool cancelled = [error.domain isEqualToString:ASAuthorizationErrorDomain] && error.code == ASAuthorizationErrorCanceled;
  [self finish:new Result{"", cancelled ? "CANCELLED" : "NATIVE_FAILED", cancelled ? "Passkey cancelled. Try again." : "The Mac could not complete this passkey request. Retry or use your password or recovery code."}];
}
- (void)authorizationController:(ASAuthorizationController*)controller didCompleteWithAuthorization:(ASAuthorization*)authorization {
  @try {
    NSMutableDictionary* response = [NSMutableDictionary dictionary];
    NSData* credentialId = nil;
    if ([authorization.credential isKindOfClass:[ASAuthorizationPlatformPublicKeyCredentialRegistration class]]) {
      auto credential = (ASAuthorizationPlatformPublicKeyCredentialRegistration*)authorization.credential;
      credentialId = credential.credentialID;
      if (!credential.rawAttestationObject.length || !credential.rawClientDataJSON.length) @throw [NSException exceptionWithName:@"Invalid" reason:nil userInfo:nil];
      response[@"clientDataJSON"] = Encode(credential.rawClientDataJSON);
      response[@"attestationObject"] = Encode(credential.rawAttestationObject);
    } else if ([authorization.credential isKindOfClass:[ASAuthorizationPlatformPublicKeyCredentialAssertion class]]) {
      auto credential = (ASAuthorizationPlatformPublicKeyCredentialAssertion*)authorization.credential;
      credentialId = credential.credentialID;
      if (!credential.rawClientDataJSON.length || !credential.rawAuthenticatorData.length || !credential.signature.length) @throw [NSException exceptionWithName:@"Invalid" reason:nil userInfo:nil];
      response[@"clientDataJSON"] = Encode(credential.rawClientDataJSON);
      response[@"authenticatorData"] = Encode(credential.rawAuthenticatorData);
      response[@"signature"] = Encode(credential.signature);
      if (credential.userID.length) response[@"userHandle"] = Encode(credential.userID);
    } else @throw [NSException exceptionWithName:@"Invalid" reason:nil userInfo:nil];
    if (!credentialId.length) @throw [NSException exceptionWithName:@"Invalid" reason:nil userInfo:nil];
    NSDictionary* object = @{@"id": Encode(credentialId), @"rawId": Encode(credentialId), @"type": @"public-key", @"authenticatorAttachment": @"platform", @"clientExtensionResults": @{}, @"response": response};
    NSData* json = [NSJSONSerialization dataWithJSONObject:object options:0 error:nil];
    if (!json || json.length > 65536) @throw [NSException exceptionWithName:@"Invalid" reason:nil userInfo:nil];
    [self finish:new Result{std::string(static_cast<const char*>(json.bytes), json.length), "", ""}];
  } @catch (NSException*) { [self finish:new Result{"", "INVALID_RESPONSE", "The Mac returned an invalid passkey response."}]; }
}
@end

static std::shared_ptr<State>& GetState(napi_env env) { void* value; napi_get_instance_data(env, &value); return *static_cast<std::shared_ptr<State>*>(value); }
static napi_value Capabilities(napi_env env, napi_callback_info info) {
  size_t argc = 1; napi_value argv[1]; napi_get_cb_info(env, info, &argc, argv, nullptr, nullptr);
  if (argc != 1) { napi_throw_type_error(env, nullptr, "A relying party domain is required."); return nullptr; }
  @autoreleasepool {
    const std::string rp = String(env, argv[0]);
    NSString* reason = Unavailable([NSString stringWithUTF8String:rp.c_str()]);
    napi_value object, available; napi_create_object(env, &object); napi_get_boolean(env, reason == nil, &available);
    napi_set_named_property(env, object, "available", available);
    if (reason) napi_set_named_property(env, object, "reason", Text(env, reason.UTF8String));
    return object;
  }
}
static napi_value Begin(napi_env env, napi_callback_info info) {
  size_t argc = 2; napi_value argv[2]; napi_get_cb_info(env, info, &argc, argv, nullptr, nullptr);
  auto state = GetState(env);
  if (argc != 2 || state->closing || ![NSThread isMainThread]) { napi_throw_error(env, nullptr, "Native passkeys require the live application main thread."); return nullptr; }
  if (@available(macOS 13.0, *)) {
    @autoreleasepool {
      if (state->active && !state->active.finished) { napi_throw_error(env, "NATIVE_BUSY", "The Mac is finishing a previous passkey request. Retry in a moment."); return nullptr; }
      const std::string input = String(env, argv[0]);
      id request = [NSJSONSerialization JSONObjectWithData:[NSData dataWithBytes:input.data() length:input.size()] options:0 error:nil];
      bool buffer = false; void* pointer = nullptr; size_t bytes = 0;
      napi_is_buffer(env, argv[1], &buffer);
      if (buffer) napi_get_buffer_info(env, argv[1], &pointer, &bytes);
      if (![request isKindOfClass:[NSDictionary class]] || bytes != sizeof(NSView*)) { napi_throw_type_error(env, nullptr, "Invalid native passkey request."); return nullptr; }
      NSString* rp = request[@"rpId"]; NSString* operationId = request[@"operationId"]; NSString* kind = request[@"kind"];
      NSData* challenge = Decode(request[@"challenge"]);
      NSData* user = Decode(request[@"userId"]); NSData* idBytes = Decode(request[@"credentialId"]);
      if (![rp isKindOfClass:[NSString class]] || ![kind isKindOfClass:[NSString class]] || ![operationId isKindOfClass:[NSString class]] || operationId.length != 32 || !challenge.length || challenge.length > 64 ||
          (!([kind isEqualToString:@"register"] && user.length && user.length <= 64 && [request[@"userName"] isKindOfClass:[NSString class]]) && !([kind isEqualToString:@"authenticate"] && idBytes.length && idBytes.length <= 2048))) {
        napi_throw_type_error(env, nullptr, "Invalid native passkey request."); return nullptr;
      }
      NSString* reason = Unavailable(rp);
      if (reason) { napi_throw_error(env, "APP_NOT_PROVISIONED", reason.UTF8String); return nullptr; }
      NSView* __unsafe_unretained view = nil; memcpy(&view, pointer, sizeof(view));
      NSWindow* window = view.window;
      if (!window || !window.visible) { napi_throw_error(env, nullptr, "Show Simplex before using a passkey."); return nullptr; }
      napi_value promise; napi_deferred deferred; napi_create_promise(env, &deferred, &promise);
      auto* completion = new Completion{state, deferred, operationId.UTF8String};
      SimplexPasskeyOperation* operation = [SimplexPasskeyOperation new];
      operation.operationId = operationId; operation.window = window; operation.completion = completion;
      napi_threadsafe_function callback;
      if (napi_create_threadsafe_function(env, nullptr, nullptr, Text(env, "Simplex passkey"), 0, 1, completion, Finalize, completion, Deliver, &callback) != napi_ok) {
        delete completion; napi_throw_error(env, nullptr, "Could not initialize native passkey completion."); return nullptr;
      }
      operation.callback = callback;
      state->active = operation;
      @try {
        ASAuthorizationPlatformPublicKeyCredentialProvider* provider = [[ASAuthorizationPlatformPublicKeyCredentialProvider alloc] initWithRelyingPartyIdentifier:rp];
        ASAuthorizationRequest* authorizationRequest;
        if ([kind isEqualToString:@"register"]) {
          auto registration = [provider createCredentialRegistrationRequestWithChallenge:challenge name:request[@"userName"] userID:user];
          registration.userVerificationPreference = ASAuthorizationPublicKeyCredentialUserVerificationPreferenceRequired;
          authorizationRequest = registration;
        } else {
          auto assertion = [provider createCredentialAssertionRequestWithChallenge:challenge];
          assertion.userVerificationPreference = ASAuthorizationPublicKeyCredentialUserVerificationPreferenceRequired;
          assertion.allowedCredentials = @[[[ASAuthorizationPlatformPublicKeyCredentialDescriptor alloc] initWithCredentialID:idBytes]];
          authorizationRequest = assertion;
        }
        operation.controller = [[ASAuthorizationController alloc] initWithAuthorizationRequests:@[authorizationRequest]];
        operation.controller.delegate = operation; operation.controller.presentationContextProvider = operation;
        [operation.controller performRequests];
      } @catch (NSException*) { [operation finish:new Result{"", "NATIVE_FAILED", "The Mac could not start the passkey request."}]; }
      return promise;
    }
  }
  napi_throw_error(env, nullptr, "Native passkeys require macOS 13 or later."); return nullptr;
}
static napi_value Cancel(napi_env env, napi_callback_info info) {
  size_t argc = 1; napi_value argv[1]; napi_get_cb_info(env, info, &argc, argv, nullptr, nullptr);
  if (@available(macOS 13.0, *)) {
    auto state = GetState(env);
    if (argc == 1 && [NSThread isMainThread] && state->active && String(env, argv[0]) == std::string(state->active.operationId.UTF8String)) [state->active cancel];
  }
  napi_value undefined; napi_get_undefined(env, &undefined); return undefined;
}
static void Cleanup(void* data) {
  auto state = *static_cast<std::shared_ptr<State>*>(data);
  state->closing = true;
  if (@available(macOS 13.0, *)) { [state->active cancel]; state->active = nil; }
}
static napi_value Initialize(napi_env env, napi_value exports) {
  auto* holder = new std::shared_ptr<State>(std::make_shared<State>());
  napi_set_instance_data(env, holder, [](napi_env, void* data, void*) { delete static_cast<std::shared_ptr<State>*>(data); }, nullptr);
  napi_add_env_cleanup_hook(env, Cleanup, holder);
  napi_property_descriptor descriptors[] = {{"capabilities", nullptr, Capabilities, nullptr, nullptr, nullptr, napi_default, nullptr}, {"begin", nullptr, Begin, nullptr, nullptr, nullptr, napi_default, nullptr}, {"cancel", nullptr, Cancel, nullptr, nullptr, nullptr, napi_default, nullptr}};
  napi_define_properties(env, exports, 3, descriptors);
  return exports;
}
NAPI_MODULE(NODE_GYP_MODULE_NAME, Initialize)
