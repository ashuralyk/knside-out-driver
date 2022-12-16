import Client from "../tiktok";

let client = new Client(
  "http://127.0.0.1:8090",
  "0x8d929e962f940f75aa32054f19a5ea2ce70ae30bfe4ff7cf2dbed70d556265df",
  "ckt1qyq93wzur9h9l6qwyk6d4dvkuufp6gvl08aszz5syl"
);

async function purchase_box() {
  let tx_hash = await client.purchase_box();
  console.log(tx_hash);
}

purchase_box();

/**
 * Example:
0x444f69f07d2bf1a5855481c424f6c2a9ad60f7a22ae0635f958705e0b84135ee
 */
