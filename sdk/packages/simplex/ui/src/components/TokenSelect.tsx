import * as Select from "@radix-ui/react-select"
import { CheckIcon, ChevronDownIcon } from "./InterfaceIcons"
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
	const selectedValue = custom ? CUSTOM_TOKEN_VALUE : value || undefined

	return (
		<Select.Root
			value={selectedValue}
			onValueChange={(next) => (next === CUSTOM_TOKEN_VALUE ? onCustom() : onSelect(next))}
		>
			<Select.Trigger className="token-select-trigger" aria-label={label} aria-required="true">
				<span className="token-select-current">
					<TokenIcon symbol={value} />
					<Select.Value placeholder="Choose token" />
				</span>
				<Select.Icon className="token-select-chevron">
					<ChevronDownIcon aria-hidden="true" />
				</Select.Icon>
			</Select.Trigger>
			<Select.Content className="token-select-content" position="popper" sideOffset={6} collisionPadding={12}>
				<Select.Viewport className="token-select-viewport">
					{symbols.map((symbol) => (
						<Select.Item className="token-select-item" key={symbol} value={symbol}>
							<TokenIcon symbol={symbol} />
							<Select.ItemText>{symbol}</Select.ItemText>
							<Select.ItemIndicator className="token-select-indicator">
								<CheckIcon aria-hidden="true" />
							</Select.ItemIndicator>
						</Select.Item>
					))}
					<Select.Separator className="token-select-separator" />
					<Select.Item className="token-select-item token-select-custom" value={CUSTOM_TOKEN_VALUE}>
						<TokenIcon symbol="" />
						<Select.ItemText>Custom token</Select.ItemText>
						<Select.ItemIndicator className="token-select-indicator">
							<CheckIcon aria-hidden="true" />
						</Select.ItemIndicator>
					</Select.Item>
				</Select.Viewport>
			</Select.Content>
		</Select.Root>
	)
}
