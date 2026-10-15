import { SubstrateEvent } from "@subql/types"
import { wrap } from "@/utils/event.utils"
import { GovernanceService } from "@/services/governance.service"

export const handleReferendaEvent = wrap((event: SubstrateEvent) => GovernanceService.handleReferendaEvent(event))
