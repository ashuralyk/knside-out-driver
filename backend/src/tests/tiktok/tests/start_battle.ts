import Client from "../tiktok";

let client = new Client(
  "http://127.0.0.1:8090",
  "0x8d929e962f940f75aa32054f19a5ea2ce70ae30bfe4ff7cf2dbed70d556265df",
  "ckt1qyq93wzur9h9l6qwyk6d4dvkuufp6gvl08aszz5syl"
);

async function start_battle(card_outpoint1, card_outpoint2) {
  let tx_hash = await client.start_tiktok_battle(
    card_outpoint1,
    card_outpoint2
  );
  let tx_hash_copy = await client.wait_transaction_committed(tx_hash);
  console.log(tx_hash_copy);
}

start_battle(
  {
    tx_hash:
      "0x8361b1d1491c45521ce8361370d57c8c220287a8205367c528e81ae577a8f481", // refer to get_cards outpoint.tx_hash
    index: 1,
  },
  {
    tx_hash:
      "0x8361b1d1491c45521ce8361370d57c8c220287a8205367c528e81ae577a8f481", // refer to get_cards outpoint.tx_hash
    index: 2,
  }
);

/**
 * Example:
0x991de9ee86ef96d106e66bd78cdae7651700e6caffcba9bf78e46cec9a08b99d
 */
