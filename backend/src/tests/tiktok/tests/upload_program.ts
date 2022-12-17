import Client from "../tiktok";

let client = new Client(
  "http://127.0.0.1:8090",
  "0x8d929e962f940f75aa32054f19a5ea2ce70ae30bfe4ff7cf2dbed70d556265df",
  "ckt1qyq93wzur9h9l6qwyk6d4dvkuufp6gvl08aszz5syl"
);

async function upload_program(card_outpoint, program) {
  let tx_hash: string | void = await client
    .upload_card_program(card_outpoint, program)
    .catch(console.log);
  if (tx_hash !== undefined) {
    let tx_hash_copy = await client.wait_transaction_committed(
      tx_hash as string
    );
    console.log(tx_hash_copy);
  }
}

upload_program(
  {
    tx_hash:
      "0xd3cda19db6fdf8503c0dc75855bd08ba1490d039337e690e9a403d6acdf6dfee", // refer to get_cards outpoint.tx_hash
    index: 1,
  },
  "\"return function(r) print('round: ' .. r) end\""
);

/**
 * Example:
0xd409d3466cf2a727468aa93314f2377597f1693c3cf4dbcd600455ac2e459fbd
 */
