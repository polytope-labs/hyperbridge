import { SubstrateEvent } from "@subql/types"
import { wrap } from "@/utils/event.utils"
import { GovernanceService } from "@/services/governance.service"

export const handleTechnicalCollectiveEvent = wrap((event: SubstrateEvent) =>
	GovernanceService.handleTechnicalCollectiveEvent(event),
)
