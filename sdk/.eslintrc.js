module.exports = {
	root: true,
	parser: "@typescript-eslint/parser",
	plugins: ["@typescript-eslint"],
	extends: ["eslint:recommended", "plugin:@typescript-eslint/recommended", "prettier"],
	env: {
		node: true,
		browser: true,
		es6: true,
	},
	parserOptions: {
		ecmaVersion: 2020,
		sourceType: "module",
		project: ["./tsconfig.base.json", "./packages/*/tsconfig.json"],
	},
	ignorePatterns: ["node_modules", "dist", "build", "**/*.js", "!.eslintrc.js"],
	rules: {
		"@typescript-eslint/explicit-module-boundary-types": "off",
		"@typescript-eslint/no-explicit-any": "warn",
		"@typescript-eslint/no-unused-vars": ["warn", { argsIgnorePattern: "^_" }],
		"no-console": ["warn", { allow: ["warn", "error"] }],
	},
}
