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
	// E2E tests need a live simnode + a forked EVM node; they run explicitly via test:phantom-e2e,
	// not as part of the default unit-test run.
	testPathIgnorePatterns: ["/node_modules/", "\\.e2e\\.test\\.ts$"],
	moduleNameMapper: {
		"^@/(.*)$": "<rootDir>/src/$1",
	},
	moduleFileExtensions: ["ts", "tsx", "js", "jsx", "json", "node"],
	testTimeout: 30000,
}
