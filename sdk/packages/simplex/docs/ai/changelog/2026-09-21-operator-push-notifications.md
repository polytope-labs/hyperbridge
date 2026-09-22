# Operator push notifications

Simplex now keeps notification rules and Web Push subscriptions in durable operator state. Operators can
set a spendable USD-stable liquidity threshold, opt into completed-swap alerts, enable or disable the
current PWA device, and send a test notification from Operations → Notifications.

On load, the PWA reconciles its browser subscription with the running solver and rejects subscriptions
created for a different VAPID key. Test delivery reports an error when the selected browser or desktop
shell cannot confirm delivery; native desktop tests wait for Electron to hand the alert to the operating
system notification API and emit its native shown event before reporting success.

Low-liquidity alerts fire once per downward threshold crossing and do not treat missing balance data as a
zero balance. The PWA service worker receives background pushes and opens the relevant dashboard page when
an alert is selected. The native desktop app consumes the same solver alert stream over its private socket,
including while its window is hidden or it was launched at login without a window. Each Simplex data
directory owns a persisted VAPID keypair; stale browser subscriptions are removed after a push service
reports them expired.
