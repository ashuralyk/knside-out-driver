
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
    assert(KOC.owner == KOC.inputs[1].owner, "only owner")
    assert(KOC.candidates[1], "non-empty candidates")
    return {
        driver = KOC.candidates[1]
    }
end

function deposit(quantity)
    assert(KOC.owner == KOC.inputs[1].owner, "only owner")
    assert(KOC.ckb_deposit(quantity), "deposit: sufficient ckbytes")
end

function withdraw(quantity)
    assert(KOC.owner == KOC.inputs[1].owner, "only owner")
    assert(KOC.ckb_withdraw(quantity), "withdraw: sufficient ckbytes")
end

function battle_win()
    -- assert(KOC.ckb_deposit(200), "deposit: sufficient ckbytes of 200")
    local sender = KOC.inputs[1].owner
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

    return {
        global = global
    }
end

function battle_lose()
    local sender = KOC.inputs[1].owner
    local global = KOC.global

    global.battle_count = global.battle_count + 1
    local value = global.personals[sender] or {
        win_count = 0,
        lose_count = 0,
        nfts = {}
    }
    value.lose_count = value.lose_count + 1
    global.personals[sender] = value

    return {
        global = global
    }
end

function claim_nfts()
    assert(KOC.ckb_deposit(200), "deposit: sufficient ckbytes of 200")
    local sender = KOC.inputs[1].owner
    local global = KOC.global
    local personal = assert(global.personals[sender], 'no sender')
    local nfts = personal.nfts or {}
    personal.nfts = {}
    assert(#nfts > 0, "empty nfts")

    local data = KOC.inputs[1].data or {}
    for _, nft_id in ipairs(nfts) do
        table.insert(data, nft_id)
    end
    
    return {
        global = global,
        outputs = {
            { owner = sender, data = data }
        }
    }
end
