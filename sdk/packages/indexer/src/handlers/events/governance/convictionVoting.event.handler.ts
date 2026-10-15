import { SubstrateEvent } from "@subql/types"
import { wrap } from "@/utils/event.utils"
import { GovernanceService } from "@/services/governance.service"

export const handleConvictionVotingEvent = wrap((event: SubstrateEvent) =>
	GovernanceService.handleConvictionVotingEvent(event),
)
