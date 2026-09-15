export const NODE_VERSION: string
export const NODE_BASE_URL: string
export const NODE_RELEASE_KEY_FINGERPRINT: string
export const TARGETS: string[]
export function assetFor(target: string): string
export function checksumFromManifest(manifest: string, asset: string): string
export function verifyManifestSignature(
	signedManifest: string,
	armoredKey: string,
	expectedFingerprint?: string,
): Promise<string>
export function extractNode(archive: string, destination: string): Promise<void>
export function atomicInstall(source: string, destination: string, executable: boolean): Promise<void>
export function mergeUniversal(
	arm64: string,
	x64: string,
	destination: string,
	spawnImpl?: (...args: unknown[]) => { status: number | null; stdout?: string; stderr?: string },
): void
export function defaultTarget(platform?: NodeJS.Platform, arch?: string): string
export function stageNode(target: string, outputRoot?: string): Promise<string>
