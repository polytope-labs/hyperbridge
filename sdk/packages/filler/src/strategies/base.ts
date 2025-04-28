import { Order, FillerConfig, ExecutionResult } from "hyperbridge-sdk"
export interface FillerStrategy {
	name: string

	canFill(order: Order): Promise<boolean>

	calculateProfitability(order: Order): Promise<bigint>

	executeOrder(order: Order): Promise<ExecutionResult>
}
