import Client from "../tiktok";

let client = new Client(
  "http://127.0.0.1:8090",
  "0x8d929e962f940f75aa32054f19a5ea2ce70ae30bfe4ff7cf2dbed70d556265df",
  "ckt1qyq93wzur9h9l6qwyk6d4dvkuufp6gvl08aszz5syl"
);

async function get_boxes() {
  let boxes = await client.get_boxes();
  console.log(boxes);
}

get_boxes();

/**
 * Example
[
  {
    data: { box_id: 6, max_cards: 1 },
    outpoint: {
      tx_hash: '0xa059aee8f8d0c71a0910afe468b9193f2a47305ca29bccbaf7f8bbfd9b25a439',
      index: 1
    }
  },
  {
    data: { max_cards: 2, box_id: 7 },
    outpoint: {
      tx_hash: '0xb2a10a0ef4980eff44ef85a0dc17d7728e721ff6571463eda5857b45dc48396f',
      index: 1
    }
  },
  {
    data: { max_cards: 3, box_id: 8 },
    outpoint: {
      tx_hash: '0x1e2eb6fcfc111e55fdb6db87ace77c9f05a235addb5d0cab094e8d0930e862ab',
      index: 1
    }
  },
  {
    data: { box_id: 9, max_cards: 2 },
    outpoint: {
      tx_hash: '0xd9f053761663e8fe59e801b9990c9fc1ff3677ea6c316acf9a7cb275b53f43f4',
      index: 1
    }
  }
]
 */
