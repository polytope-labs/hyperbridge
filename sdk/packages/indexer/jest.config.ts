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
	testTimeout: 30000,
}
