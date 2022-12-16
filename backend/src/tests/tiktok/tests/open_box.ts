import Client from "../tiktok";

let client = new Client(
  "http://127.0.0.1:8090",
  "0x8d929e962f940f75aa32054f19a5ea2ce70ae30bfe4ff7cf2dbed70d556265df",
  "ckt1qyq93wzur9h9l6qwyk6d4dvkuufp6gvl08aszz5syl"
);

async function open_box(box_outpoint) {
  let tx_hash = await client.open_box(box_outpoint);
  let tx_hash_copy = await client.wait_transaction_committed(tx_hash);
  console.log(tx_hash_copy);
}

open_box({
  tx_hash: "0xa059aee8f8d0c71a0910afe468b9193f2a47305ca29bccbaf7f8bbfd9b25a439", // refer to get_boxes outpoint.tx_hash
  index: 1,
});

/**
 * Example:
0x8056cafe85283e1451c67893343b667e957d61466ecc42acaf107368898ba556
 */
