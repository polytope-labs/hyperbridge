const { existsSync, readFileSync } = require("node:fs")
const { resolve } = require("node:path")
const { parse } = require("yaml")

const packageRoot = resolve(__dirname, "..")
const ENTITLEMENTS_PATH = "resources/entitlements.mac.plist"
const MAC_RUNTIME_PATH = "Contents/Resources/runtime/node"
const AZURE_HOST_SUFFIX = ".codesigning.azure.net"

function releaseSigningEnabled(env = process.env) {
	const value = env.SIMPLEX_DESKTOP_SIGN_RELEASE
	if (value == null || value === "" || value === "false") return false
	if (value === "true") return true
	throw new Error("SIMPLEX_DESKTOP_SIGN_RELEASE must be exactly 'true' or 'false'")
}

function requireEnvironment(env, names, platform) {
	const missing = names.filter((name) => !env[name]?.trim())
	if (missing.length > 0) {
		throw new Error(`Signed ${platform} releases require: ${missing.join(", ")}`)
	}
}

function validateAzureEndpoint(value) {
	let endpoint
	try {
		endpoint = new URL(value)
	} catch {
		throw new Error("SIMPLEX_AZURE_SIGNING_ENDPOINT must be a valid HTTPS URL")
	}
	if (
		endpoint.protocol !== "https:" ||
		!endpoint.hostname.endsWith(AZURE_HOST_SUFFIX) ||
		endpoint.username ||
		endpoint.password ||
		endpoint.search ||
		endpoint.hash
	) {
		throw new Error(
			`SIMPLEX_AZURE_SIGNING_ENDPOINT must be an HTTPS ${AZURE_HOST_SUFFIX} endpoint without credentials, query, or fragment`,
		)
	}
	return endpoint.href
}

function assertReleaseSigningEnvironment(platform = process.platform, env = process.env) {
	if (!releaseSigningEnabled(env)) return { enabled: false, platform }

	if (platform === "darwin") {
		requireEnvironment(
			env,
			[
				"CSC_LINK",
				"CSC_KEY_PASSWORD",
				"APPLE_API_KEY",
				"APPLE_API_KEY_ID",
				"APPLE_API_ISSUER",
				"APPLE_TEAM_ID",
			],
			"macOS",
		)
		if (!existsSync(env.APPLE_API_KEY)) {
			throw new Error(`APPLE_API_KEY does not exist: ${env.APPLE_API_KEY}`)
		}
		return { enabled: true, platform, teamId: env.APPLE_TEAM_ID }
	}

	if (platform === "win32") {
		requireEnvironment(
			env,
			[
				"AZURE_TENANT_ID",
				"AZURE_CLIENT_ID",
				"AZURE_CLIENT_SECRET",
				"SIMPLEX_WINDOWS_PUBLISHER_NAME",
				"SIMPLEX_AZURE_SIGNING_ENDPOINT",
				"SIMPLEX_AZURE_SIGNING_ACCOUNT_NAME",
				"SIMPLEX_AZURE_CERTIFICATE_PROFILE_NAME",
			],
			"Windows",
		)
		return {
			enabled: true,
			platform,
			publisherName: env.SIMPLEX_WINDOWS_PUBLISHER_NAME,
			endpoint: validateAzureEndpoint(env.SIMPLEX_AZURE_SIGNING_ENDPOINT),
		}
	}

	if (platform === "linux") return { enabled: true, platform }
	throw new Error(`Unsupported desktop signing platform: ${platform}`)
}

function loadBuilderConfig(env = process.env, platform = process.platform) {
	const config = parse(readFileSync(resolve(packageRoot, "electron-builder.yml"), "utf8"))
	const signing = assertReleaseSigningEnvironment(platform, env)

	if (!signing.enabled) {
		config.mac = { ...config.mac, identity: null, notarize: false }
		config.win = { ...config.win, signExecutable: false }
		return config
	}

	if (platform === "darwin") {
		config.forceCodeSigning = true
		config.dmg = { ...config.dmg, sign: true }
		// This public identifier is sealed inside app.asar. The runtime updater
		// uses it to require the same Team ID that signed the release artifact.
		config.extraMetadata = { ...config.extraMetadata, simplexMacTeamId: signing.teamId }
		config.mac = {
			...config.mac,
			hardenedRuntime: true,
			entitlements: ENTITLEMENTS_PATH,
			entitlementsInherit: ENTITLEMENTS_PATH,
			preAutoEntitlements: false,
			strictVerify: true,
			binaries: [MAC_RUNTIME_PATH],
			notarize: true,
		}
	}

	if (platform === "win32") {
		config.forceCodeSigning = true
		config.win = {
			...config.win,
			signExecutable: true,
			signExts: [".exe"],
			azureSignOptions: {
				publisherName: signing.publisherName,
				endpoint: signing.endpoint,
				certificateProfileName: env.SIMPLEX_AZURE_CERTIFICATE_PROFILE_NAME,
				codeSigningAccountName: env.SIMPLEX_AZURE_SIGNING_ACCOUNT_NAME,
				fileDigest: "SHA256",
				timestampDigest: "SHA256",
				timestampRfc3161: "http://timestamp.acs.microsoft.com",
			},
		}
	}

	return config
}

async function signPackagedWindowsRuntime(runtimePath, context, env = process.env) {
	if (!releaseSigningEnabled(env) || context.electronPlatformName !== "win32") return false
	if (typeof context.packager.signIf !== "function") {
		throw new Error("electron-builder cannot sign the staged Windows Node runtime")
	}
	const signed = await context.packager.signIf(runtimePath)
	if (!signed) throw new Error("electron-builder did not sign the staged Windows Node runtime")
	return true
}

module.exports = {
	ENTITLEMENTS_PATH,
	MAC_RUNTIME_PATH,
	assertReleaseSigningEnvironment,
	loadBuilderConfig,
	releaseSigningEnabled,
	signPackagedWindowsRuntime,
	validateAzureEndpoint,
}

if (require.main === module) {
	const result = assertReleaseSigningEnvironment()
	process.stdout.write(
		result.enabled
			? `Validated signed Simplex desktop release environment for ${result.platform}\n`
			: `Simplex desktop signing is disabled for this ${result.platform} package\n`,
	)
}
