import Axios from "axios";

interface Box {
  box_id: number;
  max_cards: number;
}

interface Card {
  id: number;
  level: number;
  rarity: string;
  weapon: string;
  skill: string;
  race: string;
  tribe: string;
  program?: string;
}

interface Outpoint {
  tx_hash: string;
  index: number;
}

interface RawOutpoint {
  tx_hash: string;
  index: string;
}

interface PerosnalItem {
  data: Box | Card;
  outpoint: Outpoint;
}

interface PerosnalRawItem {
  data: string;
  outpoint: RawOutpoint;
}

type TxHash = string;

export default class Client {
  url: string;
  privkey: string;
  address: string;
  project_typeargs: string;

  constructor(url: string, privkey: string, address: string) {
    this.url = url;
    this.privkey = privkey;
    this.address = address;
    this.project_typeargs =
      "0xfc03b799cd921255f48aaf28f36d613d8addfd8b3dadbc945d94f21a3d00a67b";
  }

  private async request(method: string, param: any): Promise<any> {
    return await Axios.post(
      this.url,
      {
        jsonrpc: "2.0",
        method,
        params: param,
        id: 1,
      },
      {
        headers: {
          "content-type": "application/json; charset=utf-8",
        },
      }
    );
  }

  private async make_transaction_digest(
    call_func: string,
    outpoints: Array<Outpoint> | null
  ): Promise<string> {
    let sender: string | null = this.address;
    if (outpoints !== null) {
      sender = null;
    }
    let response = await this.request("ko_makeRequestTransactionDigest", {
      contract_call: call_func,
      sender: sender,
      inputs: outpoints,
      candidates: [],
      components: [],
      project_type_args: this.project_typeargs,
    });
    console.log(response);
    if (response.status != 200 || response.data.result === undefined) {
      throw "bad jsonrpc call";
    }
    return response.data.result;
  }

  private async send_transaction(digest: string): Promise<TxHash> {
    let signature = "0xaadadsd"; // todo
    let response = await this.request("ko_sendTransactionSignature", {
      digest,
      signature,
    });
    if (response.status != 200 || response.data.result === undefined) {
      throw "bad jsonrpc call";
    }
    return response.data.result;
  }

  private async fetch_perosnal_data(): Promise<Array<PerosnalRawItem>> {
    let response = await this.request("ko_fetchPersonalData", {
      address: this.address,
      project_type_args: this.project_typeargs,
    });
    if (response.status != 200 || response.data.result === undefined) {
      throw "bad jsonrpc call";
    }
    return response.data.result.data;
  }

  public async wait_transaction_committed(
    hash: TxHash
  ): Promise<TxHash | null> {
    let response = await this.request("ko_waitRequestTransactionCommitted", {
      request_hash: hash,
      project_type_args: this.project_typeargs,
    });
    if (response.status != 200 || response.data.result === undefined) {
      throw "bad jsonrpc call";
    }
    return response.data.result;
  }

  public async purchase_box(): Promise<TxHash> {
    let digest = await this.make_transaction_digest("purchase_box()", null);
    return await this.send_transaction(digest);
  }

  public async open_box(box: Outpoint): Promise<TxHash> {
    let digest = await this.make_transaction_digest("open_box()", [box]);
    return await this.send_transaction(digest);
  }

  public async upload_card_program(
    card: Outpoint,
    program: string
  ): Promise<TxHash> {
    let digest = await this.make_transaction_digest(
      `set_card_program(${program})`,
      [card]
    );
    return await this.send_transaction(digest);
  }

  public async start_tiktok_battle(
    card1: Outpoint,
    card2: Outpoint
  ): Promise<TxHash> {
    let digest = await this.make_transaction_digest(`start_tiktok_battle()`, [
      card1,
      card2,
    ]);
    return await this.send_transaction(digest);
  }

  public async get_boxes(): Promise<Array<PerosnalItem>> {
    let personal_items = await this.fetch_perosnal_data();
    return personal_items
      .map((item) => {
        let nft = JSON.parse(item.data);
        if (nft.box_id !== undefined) {
          return {
            data: nft as Box,
            outpoint: {
              tx_hash: item.outpoint.tx_hash,
              index: parseInt(item.outpoint.index, 16),
            },
          };
        } else {
          return null;
        }
      })
      .filter((item) => item !== null);
  }

  public async get_cards(): Promise<Array<PerosnalItem>> {
    let personal_items = await this.fetch_perosnal_data();
    return personal_items
      .map((item) => {
        let nft = JSON.parse(item.data);
        if (nft.id !== undefined) {
          return {
            data: nft as Card,
            outpoint: {
              tx_hash: item.outpoint.tx_hash,
              index: parseInt(item.outpoint.index, 16),
            },
          };
        } else {
          return null;
        }
      })
      .filter((item) => item !== null);
  }
}

/**
 * FOR EXAMPLES
 *
 *  let client = new Client(
 *      'http://127.0.0.1:8090',
 *      '0x8d929e962f940f75aa32054f19a5ea2ce70ae30bfe4ff7cf2dbed70d556265df',
 *      'ckt1qyq93wzur9h9l6qwyk6d4dvkuufp6gvl08aszz5syl'
 *  );
 *  let tx_hash = await client.purchase_box();
 */
