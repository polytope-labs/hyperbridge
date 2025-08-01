import { __test } from '@hyperbridge/sdk';

it("should load test function and invoke without failure", async () => {
	expect(await __test()).toMatchInlineSnapshot(`"{"root":"0x85835c4d5287fe023073eb733f4e4103935d61b4397f0f9d0fe627d434757fa5","proof":["0x894376e04f932deadc9ab212ac514f37b41e670be2f8002babde1faf20935461","0xf3ace1896f86f91627cc1c09eeaba2cd76d82a75be6f09b94c861524fa5e5289","0x0ffa0900c838d17341df2d00fa4832755de619e646137844700668ad544c8aae","0xc5d2460186f7233c927e7db2dcc703c0e500b653ca82273b7bfad8045d85a470","0x9c6b2c1b0d0b25a008e6c882cc7b415f309965c72ad2b944ac0931048ca31cd5","0xfadbd3c7f79fa2bdc4f24857709cd4a4e870623dc9e9abcdfd6e448033e35212"],"mmr_size":236,"leaf_positions":[232],"keccak_hash_calldata":"0xc5d2460186f7233c927e7db2dcc703c0e500b653ca82273b7bfad8045d85a470"}"`);
})
