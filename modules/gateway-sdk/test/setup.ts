import { teleport } from '@src/index'
import { TeleportParams } from '@src/types'
import chai from 'chai'
import { ethers } from 'ethers'
import Mocha from 'mocha'
import proxyquire from 'proxyquire'
import sinon = require('sinon')
import sinonChai from 'sinon-chai'

const should = chai.should()
const expect = chai.expect

chai.use(sinonChai)

export { should, expect, Mocha, proxyquire, sinon }

const provider = new ethers.JsonRpcProvider(process.env.PROVIDER);
const privateKey = process.env.ACCOUNT_PRIVATE_KEY;
const wallet = new ethers.Wallet(`0x${privateKey}`, provider);



describe('Token Gateway', async () => {
    describe('Test for token gateway', async () => {
      it('Teleport function should bridge token', async () => {
       
        let transportParam: TeleportParams = {
            feeToken: "0x6df8dE86458D15a3Be3A6B907e6aE6B7af352452",
            amount: 10000000000000,
            redeem: false,
            dest: "0x42415345",
            fee: 692054112492000,
            timeout: 2000,
            to: "0x8731fA26aA8f75eB12aA9cf55275a9486e9e90A2",
            tokenId: "0x829f01563df2ff9752a529f62c33a4b03b805da1e1dfc748127d6d37795d7257"
        }
        let teleportResult = await teleport(wallet ,transportParam, true);
        let res = await teleportResult.wait()

        console.log(teleportResult.hash)
        console.log(res, "The response")
      })
    })


})