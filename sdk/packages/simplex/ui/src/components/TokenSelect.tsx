import { AppSelect, type AppSelectOption } from "./AppSelect"
import { TokenIcon } from "./TokenIcon"

const CUSTOM_TOKEN_VALUE = "__custom_token__"

export function TokenSelect(props: {
	label: string
	value: string
	symbols: string[]
	custom: boolean
	onSelect: (symbol: string) => void
	onCustom: () => void
}) {
	const { label, value, symbols, custom, onSelect, onCustom } = props
	const selectedValue = custom ? CUSTOM_TOKEN_VALUE : value
	const options: AppSelectOption[] = symbols.map((symbol) => ({
		value: symbol,
		label: symbol,
		leading: <TokenIcon symbol={symbol} />,
	}))
	options.push({
		value: CUSTOM_TOKEN_VALUE,
		label: "Custom token",
		leading: <TokenIcon symbol="" />,
		muted: true,
		separatorBefore: true,
	})

	return (
		<AppSelect
			value={selectedValue}
			options={options}
			onValueChange={(next) => (next === CUSTOM_TOKEN_VALUE ? onCustom() : onSelect(next))}
			placeholder="Choose token"
			ariaLabel={label}
			required
		/>
	)
}
