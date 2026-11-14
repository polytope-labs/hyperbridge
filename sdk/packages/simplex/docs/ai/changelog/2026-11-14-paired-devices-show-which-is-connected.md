# Paired devices show which one is connected

`GET /api/tunnel` reports `connected` on each entry of `devices`: whether a session that logged in
with that device's key is open right now. It comes from the sessions the embedded SSH server already
tracks per key, so it only counts a device after its login succeeded.

The Remote access page marks a connected device with a "Connected" badge next to its label. Its
"N connected" count is now the number of devices marked that way. `activeConnections` is unchanged
and still counts every open connection, including one that has not logged in yet.

The device returned by `POST /api/tunnel/devices` carries `connected: false`.
