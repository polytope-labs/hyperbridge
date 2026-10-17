import { SubstrateEvent } from "@subql/types"
import { wrap } from "@/utils/event.utils"
import { GovernanceService } from "@/services/governance.service"

export const handleWhitelistEvent = wrap((event: SubstrateEvent) => GovernanceService.handleWhitelistEvent(event))
