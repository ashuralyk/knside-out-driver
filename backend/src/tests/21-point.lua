
function construct ()
    assert(KOC.owner == KOC.driver, "only owner")
    return {
        driver = KOC.driver,
        global = {
            battle_count = 0,
            nft_token_id = 0,
            personals = {}
        }
    }
end

function change_driver()
    assert(KOC.owner == KOC.user, "only allow project owner")
    KOC.driver = KOC.recipient
end

function deposit(quantity)
    assert(KOC.owner == KOC.user, "only allow project owner")
    assert(KOC.ckb_deposit(quantity))
end

function withdraw(quantity)
    assert(KOC.owner == KOC.user, "only allow project owner")
    assert(KOC.ckb_withdraw(quantity), "global ckb not enough to withdraw")
end

function battle_win()
    -- assert(KOC.ckb_deposit(200), "claim_nft() ckb not enough")
    local sender = KOC.user
    local global = KOC.global
    global.battle_count = global.battle_count + 1
    local value = global.personals[sender] or {
        win_count = 0,
        lose_count = 0,
        nfts = {}
    }
    value.win_count = value.win_count + 1
    table.insert(value.nfts, global.nft_token_id)
    global.nft_token_id = global.nft_token_id + 1
    global.personals[sender] = value
end

function battle_lose()
    local sender = KOC.user
    local global = KOC.global
    global.battle_count = global.battle_count + 1
    local value = global.personals[sender] or {
        win_count = 0,
        lose_count = 0,
        nfts = {}
    }
    value.lose_count = value.lose_count + 1
    global.personals[sender] = value
end

function claim_nfts()
    -- assert(KOC.ckb_deposit(200), "claim_nft() ckb not enough")
    local personal = assert(KOC.global.personals[KOC.user], 'no user')
    local nfts = personal.nfts or {}
    assert(#nfts > 0, "no nfts")
    personal.nfts = {}
    local data = KOC.personal or {}
    if #data > 0 then
        for _, nft_id in ipairs(nfts) do
            table.insert(data, nft_id)
        end
    else
        KOC.personal = nfts
    end
end
