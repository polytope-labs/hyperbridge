import { SubstrateEvent } from "@subql/types"
import { wrap } from "@/utils/event.utils"
import { GovernanceService } from "@/services/governance.service"

export const handlePreimageEvent = wrap((event: SubstrateEvent) => GovernanceService.handlePreimageEvent(event))
