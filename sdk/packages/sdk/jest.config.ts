const config = {
	preset: "ts-jest/presets/default-esm",
	testEnvironment: "node",
	testMatch: ["**/tests/**/*.test.ts"],
	testTimeout: 10000,
	setupFiles: ["./src/tests/setup.ts"],
	extensionsToTreatAsEsm: [".ts"],
	moduleNameMapper: {
		"^(\\.{1,2}/.*)\\.js$": "$1",
		"@/(.*)$": "<rootDir>/src/$1",
	},
	transform: {
		"^.+\\.tsx?$": [
			"ts-jest",
			{
				useESM: true,
			},
		],
	},
	transformIgnorePatterns: ["node_modules/(?!(graphql-request|@graphql-typed-document-node)/)"],
}

export default config
