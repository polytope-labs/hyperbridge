export const REQUEST_STATUS = `
  query RequestStatusM($hash: String!) {
  requests(
    filter: { commitment: { equalTo: $hash } }
  ) {
    nodes {
      commitment
      timeoutTimestamp
      source
      dest
      to
      from
      nonce
      body
      statusMetadata {
        nodes {
          blockHash
          blockNumber
          timestamp
          chain
          status
          transactionHash
        }
      }
    }
  }
}
`

export const STATE_MACHINE_UPDATES_BY_HEIGHT = `
query StateMachineUpdatesByHeight($statemachineId: String!, $height: Int!, $chain: String!) {
	stateMachineUpdateEvents(
		filter: {
			and: [
				{ stateMachineId: { equalTo: $statemachineId } }
				{ height: { greaterThanOrEqualTo: $height } }
				{ chain: { equalTo: $chain } }
			]
		}
		orderBy: HEIGHT_ASC
		first: 1
	) {
    nodes {
      height
      stateMachineId
      chain
      blockHash
      blockNumber
      transactionHash
    }
  }
}
`

export const STATE_MACHINE_UPDATES_BY_TIMESTAMP = `
query StateMachineUpdatesByTimestamp($statemachineId: String!, $commitmentTimestamp: BigFloat!, $chain: String!) {
	stateMachineUpdateEvents(
		filter: {
			and: [
				{ stateMachineId: { equalTo: $statemachineId } }
				{ commitmentTimestamp: { greaterThanOrEqualTo: $commitmentTimestamp } }
     			{ chain: { equalTo: $chain } }
			]
		}
		orderBy: COMMITMENT_TIMESTAMP_ASC
		first: 1
	) {
    nodes {
        height
        stateMachineId
        chain
        blockHash
        blockNumber
        transactionHash
        commitmentTimestamp
      }
    }
  }
`

export const ASSET_TELEPORTED_BY_PARAMS = `
query AssetTeleportedByParams($from: String!, $to: String!, $dest: String!, $blockNumber: Int!) {
  assetTeleporteds(
    filter: {
      and: [
        { from: { equalTo: $from } }
        { to: { equalTo: $to } }
        { dest: { includes: $dest } }
        { blockNumber: { greaterThanOrEqualTo: $blockNumber } }
      ]
    }
    orderBy: CREATED_AT_DESC
    first: 1
  ) {
    nodes {
      id
      from
      to
      amount
      dest
      commitment
      createdAt
      blockNumber
    }
  }
}
`
