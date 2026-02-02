import { OrderV2, ExecutionResult, IntentsCoprocessor } from "@hyperbridge/sdk"

export interface FillerStrategy {
	name: string

	canFill(order: OrderV2): Promise<boolean>

	calculateProfitability(order: OrderV2): Promise<number>

	executeOrder(order: OrderV2, hyperbridge?: IntentsCoprocessor): Promise<ExecutionResult>
}
