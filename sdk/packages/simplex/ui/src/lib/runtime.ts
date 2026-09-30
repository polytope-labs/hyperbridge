/** The Electron shell serves the shared renderer through this private protocol. */
export function isNativeDesktopProtocol(protocol: string): boolean {
	return protocol === "simplex:"
}
