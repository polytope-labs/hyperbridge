import { UserActivity } from "@/configs/src/types"
import { timestampToDate } from "@/utils/date.helpers"

export async function getOrCreateUser(address: string, referrer?: string, timestamp?: bigint): Promise<UserActivity> {
	const user = await UserActivity.get(address)
	if (user) {
		if (!user.referrer && referrer) {
			user.referrer = referrer
			await user.save()
		}
		return user
	}
	const newUser = UserActivity.create({
		id: address,
		referrer,
		totalOrdersPlaced: BigInt(0),
		totalFilledOrders: BigInt(0),
		totalTeleports: BigInt(0),
		totalSuccessfulTeleports: BigInt(0),
		totalOrderPlacedVolumeUSD: "0",
		totalOrderFilledVolumeUSD: "0",
		totalTeleportedVolumeUSD: "0",
		totalSuccessfulTeleportedVolumeUSD: "0",
		createdAt: timestampToDate(timestamp || BigInt(0)),
	})
	await newUser.save()
	return newUser
}
