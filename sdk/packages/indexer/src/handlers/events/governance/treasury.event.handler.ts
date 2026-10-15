import { SubstrateEvent } from "@subql/types"
import { wrap } from "@/utils/event.utils"
import { GovernanceService } from "@/services/governance.service"

export const handleTreasurySpendEvent = wrap((event: SubstrateEvent) => GovernanceService.handleTreasurySpendEvent(event))
