import Client from "../tiktok";

let client = new Client(
  "http://127.0.0.1:8090",
  "0x8d929e962f940f75aa32054f19a5ea2ce70ae30bfe4ff7cf2dbed70d556265df",
  "ckt1qyq93wzur9h9l6qwyk6d4dvkuufp6gvl08aszz5syl"
);

async function get_cards() {
  let boxes = await client.get_cards();
  console.log(boxes);
}

get_cards();

/**
 * Example:
[
  {
    data: {
      weapon: '指虎',
      skill: '治愈',
      race: '矮人',
      id: 4,
      rarity: '粗糙的',
      program: "return function(r, t) print('round: ' .. r) end",
      tribe: '轩辕',
      level: 3
    },
    outpoint: {
      tx_hash: '0x8361b1d1491c45521ce8361370d57c8c220287a8205367c528e81ae577a8f481',
      index: 1
    }
  },
  {
    data: {
      weapon: '枪',
      skill: '刺杀',
      race: '天使',
      id: 2,
      rarity: '普通的',
      program: "return function(r, t) print('round: ' .. r, t.race) end",
      tribe: '地煞',
      level: 5
    },
    outpoint: {
      tx_hash: '0x8361b1d1491c45521ce8361370d57c8c220287a8205367c528e81ae577a8f481',
      index: 2
    }
  },
  {
    data: {
      weapon: '剑',
      id: 5,
      level: 1,
      tribe: '五毒',
      race: '精灵',
      skill: '刺杀',
      rarity: '粗糙的'
    },
    outpoint: {
      tx_hash: '0xd3cda19db6fdf8503c0dc75855bd08ba1490d039337e690e9a403d6acdf6dfee',
      index: 1
    }
  },
  {
    data: {
      id: 6,
      rarity: '粗糙的',
      race: '精灵',
      level: 1,
      weapon: '忍刀',
      tribe: '部落',
      skill: '火球'
    },
    outpoint: {
      tx_hash: '0x4adc61bb464ace11b1f994acc0ab33bd5687f69b4de18d548c77a0eff83e874b',
      index: 1
    }
  },
  {
    data: {
      race: '巨魔',
      tribe: '轩辕',
      skill: '治愈',
      id: 7,
      rarity: '粗糙的',
      weapon: '枪',
      level: 1
    },
    outpoint: {
      tx_hash: '0x0fb32fab08d6c73bc9e5903f241aab1b0eaf815e06d7959845814857bc946df4',
      index: 1
    }
  },
  {
    data: {
      race: '矮人',
      tribe: '部落',
      skill: '黑洞',
      id: 8,
      rarity: '普通的',
      weapon: '指虎',
      level: 2
    },
    outpoint: {
      tx_hash: '0x0fb32fab08d6c73bc9e5903f241aab1b0eaf815e06d7959845814857bc946df4',
      index: 2
    }
  },
  {
    data: {
      race: '天使',
      tribe: '五毒',
      skill: '刺杀',
      id: 9,
      rarity: '粗糙的',
      weapon: '大锤',
      level: 1
    },
    outpoint: {
      tx_hash: '0x0fb32fab08d6c73bc9e5903f241aab1b0eaf815e06d7959845814857bc946df4',
      index: 3
    }
  }
]
 */
