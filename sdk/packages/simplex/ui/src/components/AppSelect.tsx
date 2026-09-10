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
	/** Second line under the label, in the menu only. The trigger shows `caption` instead. */
	description?: ReactNode
	/** Right-aligned value on the row, before the check indicator. */
	trailing?: ReactNode
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
	/** Second line under the current value in the trigger, for a roll-up the label cannot carry. */
	caption?: ReactNode
	/** Pinned above the options, outside the scrolling viewport. Column headings, typically. */
	header?: ReactNode
	/** Extra class on the popover, which is portalled out of the caller's own subtree. */
	contentClassName?: string
}) {
	const {
		value,
		options,
		onValueChange,
		placeholder,
		ariaLabel,
		ariaLabelledBy,
		required,
		disabled,
		caption,
		header,
		contentClassName,
	} = props
	const selected = options.find((option) => option.value === value)
	const currentValue = (
		<Select.Value className="app-select-value" placeholder={placeholder}>
			{selected?.label}
		</Select.Value>
	)

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
					{caption === undefined ? (
						currentValue
					) : (
						<span className="app-select-current-text">
							{currentValue}
							<small className="app-select-caption">{caption}</small>
						</span>
					)}
				</span>
				<Select.Icon className="app-select-chevron">
					<ChevronDownIcon aria-hidden="true" />
				</Select.Icon>
			</Select.Trigger>
			<Select.Portal>
				<Select.Content
					className={contentClassName ? `app-select-content ${contentClassName}` : "app-select-content"}
					position="popper"
					sideOffset={6}
					collisionPadding={12}
				>
					{header ? <div className="app-select-header">{header}</div> : null}
					<Select.Viewport className="app-select-viewport">
						{options.map((option) => (
							<Fragment key={option.value}>
								{option.separatorBefore ? <Select.Separator className="app-select-separator" /> : null}
								<Select.Item
									className="app-select-item"
									data-muted={option.muted || undefined}
									data-rich={option.description || option.trailing ? true : undefined}
									value={encodeValue(option.value)}
									disabled={option.disabled}
								>
									{option.leading}
									{option.description === undefined ? (
										<Select.ItemText>{option.label}</Select.ItemText>
									) : (
										<span className="app-select-item-text">
											<Select.ItemText>{option.label}</Select.ItemText>
											<small className="app-select-item-description">{option.description}</small>
										</span>
									)}
									{option.trailing === undefined ? null : (
										<span className="app-select-item-trailing">{option.trailing}</span>
									)}
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
