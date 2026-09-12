export default {
	preset: "ts-jest/presets/js-with-ts",
	testEnvironment: "node",
	transform: {
		"^.+\\.(ts|tsx)$": [
			"ts-jest",
			{
				tsconfig: "tsconfig.json",
				useESM: false,
			},
		],
	},
	testMatch: ["**/__tests__/**/*.ts?(x)", "**/?(*.)+(test).ts?(x)", "**/test/**/*.ts?(x)"],
	moduleNameMapper: {
		"^@/(.*)$": "<rootDir>/src/$1",
	},
	moduleFileExtensions: ["ts", "tsx", "js", "jsx", "json", "node"],
	// The SDK's CJS bundles `require()` ESM-only packages (p-queue -> eventemitter3, lodash-es),
	// which jest cannot parse unless they are transformed. Without this, any test importing
	// `@hyperbridge/sdk` or its `intents-helpers` sub-path fails to load at all.
	transformIgnorePatterns: ["node_modules/\\.pnpm/(?!(p-queue|p-timeout|eventemitter3|lodash-es)@)"],
	testTimeout: 30000,
}
