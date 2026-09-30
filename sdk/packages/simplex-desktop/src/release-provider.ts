import type { AppUpdater } from "electron-updater"
import {
	parseUpdateInfo,
	Provider,
	type ProviderRuntimeOptions,
	resolveFiles,
} from "electron-updater/out/providers/Provider.js"

const DEFAULT_OWNER = "polytope-labs"
const DEFAULT_REPOSITORY = "hyperbridge"
const DEFAULT_TAG_PREFIX = "simplex-desktop-v"
const RELEASES_PER_PAGE = 100
const MAX_RELEASE_PAGES = 10

interface SimplexProviderOptions {
	provider: "custom"
	updateProvider?: typeof SimplexReleaseProvider
	owner?: string
	repo?: string
	tagNamePrefix?: string
}

interface GitHubRelease {
	draft: boolean
	prerelease: boolean
	tag_name: string
	name?: string | null
	body?: string | null
}

interface ReleaseVersion {
	major: number
	minor: number
	patch: number
	beta?: number
}

interface SimplexUpdateInfo extends ReturnType<typeof parseUpdateInfo> {
	tag: string
}

const RELEASE_ARTIFACT_NAME = /^[A-Za-z0-9][A-Za-z0-9._+-]*$/

export function resolveTrustedReleaseFiles(updateInfo: SimplexUpdateInfo, releaseBase: URL) {
	const files = resolveFiles(updateInfo, releaseBase, (artifact) => {
		if (typeof artifact !== "string" || !RELEASE_ARTIFACT_NAME.test(artifact)) {
			throw new Error(`Update metadata contains an untrusted artifact path: ${String(artifact)}`)
		}
		return artifact
	})
	for (const file of files) {
		if (
			file.url.protocol !== "https:" ||
			file.url.origin !== releaseBase.origin ||
			!file.url.pathname.startsWith(releaseBase.pathname) ||
			file.url.username ||
			file.url.password ||
			file.url.search ||
			file.url.hash
		) {
			throw new Error(`Update artifact escaped its pinned GitHub release: ${file.url.href}`)
		}
	}
	return files
}

function releaseVersion(release: GitHubRelease, tagPrefix: string): ReleaseVersion | undefined {
	if (
		typeof release?.tag_name !== "string" ||
		typeof release.draft !== "boolean" ||
		typeof release.prerelease !== "boolean" ||
		release.draft ||
		!release.tag_name.startsWith(tagPrefix)
	)
		return undefined
	const match = release.tag_name.slice(tagPrefix.length).match(/^(\d+)\.(\d+)\.(\d+)(?:-beta\.(\d+))?$/)
	if (!match) return undefined
	return {
		major: Number(match[1]),
		minor: Number(match[2]),
		patch: Number(match[3]),
		beta: match[4] === undefined ? undefined : Number(match[4]),
	}
}

function compareReleaseVersions(left: ReleaseVersion, right: ReleaseVersion): number {
	for (const field of ["major", "minor", "patch"] as const) {
		if (left[field] !== right[field]) return right[field] - left[field]
	}
	if (left.beta === undefined) return right.beta === undefined ? 0 : -1
	if (right.beta === undefined) return 1
	return right.beta - left.beta
}

export function selectSimplexRelease(
	releases: GitHubRelease[],
	channel: string,
	tagPrefix = DEFAULT_TAG_PREFIX,
): GitHubRelease | undefined {
	const beta = channel === "beta"
	return releases
		.map((release) => ({ release, version: releaseVersion(release, tagPrefix) }))
		.filter(({ release, version }) => {
			if (!version) return false
			return beta
				? release.prerelease && version.beta !== undefined
				: !release.prerelease && version.beta === undefined
		})
		.sort((left, right) => compareReleaseVersions(left.version!, right.version!))[0]?.release
}

/**
 * GitHub's stock updater provider treats every release in the monorepo as a desktop release.
 * This provider selects only `simplex-desktop-v*` tags, then delegates artifact integrity and
 * installation to electron-updater using the release's normal YAML metadata.
 */
export class SimplexReleaseProvider extends Provider<SimplexUpdateInfo> {
	private readonly owner: string
	private readonly repository: string
	private readonly tagPrefix: string

	constructor(
		options: SimplexProviderOptions,
		private readonly updater: AppUpdater,
		runtimeOptions: ProviderRuntimeOptions,
	) {
		super({ ...runtimeOptions, isUseMultipleRangeRequest: false })
		this.owner = options.owner ?? DEFAULT_OWNER
		this.repository = options.repo ?? DEFAULT_REPOSITORY
		this.tagPrefix = options.tagNamePrefix ?? DEFAULT_TAG_PREFIX
	}

	async getLatestVersion(): Promise<SimplexUpdateInfo> {
		const channel = this.updater.channel === "beta" ? "beta" : "latest"
		let selected: GitHubRelease | undefined

		for (let page = 1; page <= MAX_RELEASE_PAGES && !selected; page += 1) {
			const url = new URL(
				`https://api.github.com/repos/${this.owner}/${this.repository}/releases?per_page=${RELEASES_PER_PAGE}&page=${page}`,
			)
			const raw = await this.httpRequest(url, {
				Accept: "application/vnd.github+json",
				"X-GitHub-Api-Version": "2022-11-28",
			})
			const releases = JSON.parse(raw ?? "null") as unknown
			if (!Array.isArray(releases)) throw new Error("GitHub returned an invalid Simplex release list")
			selected = selectSimplexRelease(releases as GitHubRelease[], channel, this.tagPrefix)
			if (releases.length < RELEASES_PER_PAGE) break
		}

		if (!selected) throw new Error(`No ${channel} ${this.tagPrefix} release is available`)
		const channelName = this.getCustomChannelName(channel)
		const channelFile = `${channelName}.yml`
		const downloadBase = this.releaseDownloadBase(selected.tag_name)
		const channelUrl = new URL(channelFile, downloadBase)
		const update = parseUpdateInfo(await this.httpRequest(channelUrl), channelFile, channelUrl)
		return {
			...update,
			tag: selected.tag_name,
			releaseName: update.releaseName ?? selected.name ?? undefined,
			releaseNotes: update.releaseNotes ?? selected.body ?? undefined,
		}
	}

	resolveFiles(updateInfo: SimplexUpdateInfo) {
		const releaseBase = this.releaseDownloadBase(updateInfo.tag)
		return resolveTrustedReleaseFiles(updateInfo, releaseBase)
	}

	private releaseDownloadBase(tag: string): URL {
		return new URL(
			`https://github.com/${this.owner}/${this.repository}/releases/download/${encodeURIComponent(tag)}/`,
		)
	}
}

export const SIMPLEX_UPDATE_FEED: SimplexProviderOptions = {
	provider: "custom",
	updateProvider: SimplexReleaseProvider,
	owner: DEFAULT_OWNER,
	repo: DEFAULT_REPOSITORY,
	tagNamePrefix: DEFAULT_TAG_PREFIX,
}
