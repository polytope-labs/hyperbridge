import * as Dialog from "@radix-ui/react-dialog"
import { useState } from "react"

type CopyStatus = "idle" | "copied" | "failed"

function CopyIcon() {
	return (
		<svg aria-hidden="true" viewBox="0 0 20 20" width="16" height="16">
			<path
				d="M6.75 6.75h7.5v7.5h-7.5zM4 11V4h7"
				fill="none"
				stroke="currentColor"
				strokeWidth="1.5"
				strokeLinecap="round"
				strokeLinejoin="round"
			/>
		</svg>
	)
}

export function RecoveryPhraseDialog(props: { phrase: string; onAcknowledge: () => void }) {
	const [acknowledged, setAcknowledged] = useState(false)
	const [copyStatus, setCopyStatus] = useState<CopyStatus>("idle")
	const words = props.phrase.trim().split(/\s+/)

	const copyPhrase = async () => {
		try {
			await navigator.clipboard.writeText(props.phrase)
			setCopyStatus("copied")
		} catch {
			setCopyStatus("failed")
		}
	}

	return (
		<Dialog.Root open>
			<Dialog.Portal>
				<Dialog.Overlay className="dialog-overlay" />
				<Dialog.Content className="recovery-dialog" onEscapeKeyDown={(event) => event.preventDefault()}>
					<header className="recovery-dialog-header">
						<span className="eyebrow">New Hyperbridge account</span>
						<Dialog.Title asChild>
							<h2>Back up your recovery phrase</h2>
						</Dialog.Title>
						<Dialog.Description asChild>
							<p>
								Write these words down in order and store them somewhere private. You will need them to
								recover this account.
							</p>
						</Dialog.Description>
					</header>

					<div className="recovery-warning" role="note">
						<span aria-hidden="true">!</span>
						<p>
							<strong>Keep it private.</strong> Anyone with this phrase can control the account and its
							funds.
						</p>
					</div>

					<ol className="recovery-words" aria-label="Recovery phrase words">
						{words.map((word, index) => (
							<li key={`${index}-${word}`}>
								<span aria-hidden="true">{index + 1}</span>
								<strong>{word}</strong>
							</li>
						))}
					</ol>

					<div className="recovery-copy-row">
						<button type="button" className="secondary" onClick={copyPhrase}>
							<CopyIcon />
							{copyStatus === "copied" ? "Copied" : "Copy recovery phrase"}
						</button>
						<span className="recovery-copy-status" data-state={copyStatus} role="status">
							{copyStatus === "copied"
								? "Recovery phrase copied to clipboard."
								: copyStatus === "failed"
									? "Clipboard unavailable. Write the phrase down manually."
									: ""}
						</span>
					</div>

					<label className="recovery-acknowledgement">
						<input
							type="checkbox"
							checked={acknowledged}
							onChange={(event) => setAcknowledged(event.target.checked)}
						/>
						<span>I’ve saved these words somewhere safe.</span>
					</label>

					<footer className="recovery-dialog-footer">
						<button
							type="button"
							className="primary"
							disabled={!acknowledged}
							onClick={props.onAcknowledge}
						>
							I’ve saved it
						</button>
					</footer>
				</Dialog.Content>
			</Dialog.Portal>
		</Dialog.Root>
	)
}
