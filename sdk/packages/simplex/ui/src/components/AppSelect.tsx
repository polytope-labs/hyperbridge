import * as Select from "@radix-ui/react-select"
import { Fragment, type CSSProperties, type ReactNode, useId } from "react"
import { CheckIcon, ChevronDownIcon } from "./InterfaceIcons"

const VALUE_PREFIX = "simplex-select:"

function encodeValue(value: string): string {
	return `${VALUE_PREFIX}${value}`
}

function decodeValue(value: string): string {
	return value.slice(VALUE_PREFIX.length)
}

export type AppSelectOption = {
	value: string
	label: string
	leading?: ReactNode
	disabled?: boolean
	muted?: boolean
	separatorBefore?: boolean
}

export function AppSelect(props: {
	value: string
	options: AppSelectOption[]
	onValueChange: (value: string) => void
	placeholder?: string
	ariaLabel?: string
	ariaLabelledBy?: string
	required?: boolean
	disabled?: boolean
}) {
	const { value, options, onValueChange, placeholder, ariaLabel, ariaLabelledBy, required, disabled } = props
	const selected = options.find((option) => option.value === value)

	return (
		<Select.Root
			value={encodeValue(value)}
			onValueChange={(nextValue) => onValueChange(decodeValue(nextValue))}
			disabled={disabled}
		>
			<Select.Trigger
				className="app-select-trigger"
				aria-label={ariaLabel}
				aria-labelledby={ariaLabelledBy}
				aria-required={required}
			>
				<span className="app-select-current">
					{selected?.leading}
					<Select.Value className="app-select-value" placeholder={placeholder}>
						{selected?.label}
					</Select.Value>
				</span>
				<Select.Icon className="app-select-chevron">
					<ChevronDownIcon aria-hidden="true" />
				</Select.Icon>
			</Select.Trigger>
			<Select.Portal>
				<Select.Content
					className="app-select-content"
					position="popper"
					sideOffset={6}
					collisionPadding={12}
				>
					<Select.Viewport className="app-select-viewport">
						{options.map((option) => (
							<Fragment key={option.value}>
								{option.separatorBefore ? <Select.Separator className="app-select-separator" /> : null}
								<Select.Item
									className="app-select-item"
									data-muted={option.muted || undefined}
									value={encodeValue(option.value)}
									disabled={option.disabled}
								>
									{option.leading}
									<Select.ItemText>{option.label}</Select.ItemText>
									<Select.ItemIndicator className="app-select-indicator">
										<CheckIcon aria-hidden="true" />
									</Select.ItemIndicator>
								</Select.Item>
							</Fragment>
						))}
					</Select.Viewport>
				</Select.Content>
			</Select.Portal>
		</Select.Root>
	)
}

export function SelectField(props: {
	label: ReactNode
	value: string
	options: AppSelectOption[]
	onValueChange: (value: string) => void
	placeholder?: string
	required?: boolean
	disabled?: boolean
	className?: string
	style?: CSSProperties
}) {
	const { label, className, style, ...selectProps } = props
	const labelId = useId()

	return (
		<div className={className ? `field ${className}` : "field"} style={style}>
			<span id={labelId}>{label}</span>
			<AppSelect {...selectProps} ariaLabelledBy={labelId} />
		</div>
	)
}
