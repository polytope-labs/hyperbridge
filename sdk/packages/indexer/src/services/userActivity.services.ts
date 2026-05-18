import { UserActivityV2 } from "@/configs/src/types"
import { timestampToDate } from "@/utils/date.helpers"

export async function getOrCreateUser(address: string, referrer?: string, timestamp?: bigint): Promise<UserActivityV2> {
	const user = await UserActivityV2.get(address)
	if (user) {
		if (!user.referrer && referrer) {
			user.referrer = referrer
			await user.save()
		}
		return user
	}
	const newUser = UserActivityV2.create({
		id: address,
		referrer,
		totalOrdersPlaced: BigInt(0),
		totalFilledOrders: BigInt(0),
		totalOrderPlacedVolumeUSD: "0",
		totalOrderFilledVolumeUSD: "0",
		createdAt: timestampToDate(timestamp || BigInt(0)),
	})
	await newUser.save()
	return newUser
}
