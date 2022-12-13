
---------------------------
-- contract constants
---------------------------

local WEAPON = {
    "剑",
    "枪",
    "指虎",
    "忍刀",
    "大锤"
}

local SKILL = {
    "火球",
    "治愈",
    "黑洞",
    "传送",
    "刺杀"
}

local RACE = {
    "矮人",
    "天使",
    "精灵",
    "巨魔",
    "人类"
}

local TRIBE = {
    "天灾",
    "地煞",
    "五毒",
    "轩辕",
    "部落"
}

local RARITY = {
    "粗糙的",
    "普通的",
    "优质的",
    "传奇的",
    "史诗的"
}

---------------------------
-- contract constructor
---------------------------

function construct()
    assert(KOC.owner == KOC.driver, "only owner")
    return {
        driver = KOC.driver,
        global = {
            battle_count = 0,
            nft_id = 0,
            box_id = 0,
            box_price = 300,
            box_cards = 3,
            max_rounds = 5,
        }
    }
end

---------------------------
-- contract utils
---------------------------

local sender = function ()
    return KOC.inputs[1].owner
end

local data = function (i)
    return KOC.inputs[i].data
end

local random_card_level = function (baseline)
    local rand = math.random(baseline, 100)
    if rand > 95 then
        return 5
    elseif rand > 85 then
        return 4
    elseif rand > 70 then
        return 3
    elseif rand > 50 then
        return 2
    else
        return 1
    end
end

local get_card_program_limit = function (level)
    if level == 1 then
        return 50
    elseif level == 2 then
        return 75
    elseif level == 3 then
        return 100
    elseif level == 4 then
        return 125
    else
        return 150
    end
end

local mint_card = function (baseline)
    local level = random_card_level(baseline)
    return {
        id = KOC.global.nft_id,
        level = level,
        rarity = RARITY[math.random(level)],
        weapon = WEAPON[math.random(#WEAPON)],
        skill = SKILL[math.random(#SKILL)],
        race = RACE[math.random(#RACE)],
        tribe = TRIBE[math.random(#TRIBE)],
    }
end

---------------------------
-- contract owner methods
---------------------------

function _deposit(ckb_amount)
    assert(KOC.owner == sender(), "tiktok: only owner")
    assert(KOC.ckb_deposit(ckb_amount), "tiktok: insufficient ckb")
end

function _set_some_global_param(key, value)
    assert(KOC.owner == sender(), "tiktok: only owner")
    assert(type(value) == "number", "tiktok: invalid value " .. value)
    local valid_key = string.match("box_price,box_cards,max_rounds,max_place_cards", key);
    assert(valid_key, "tiktok: invalid key " .. key)
    KOC.global[key] = value
    return {
        global = KOC.global
    }
end

---------------------------
-- contract interfaces
---------------------------

function purchase_box()
    -- assert(KOC.ckb_deposit(KOC.global.box_price), "tiktok: insufficient ckb")
    -- pay to mint a box nft
    KOC.global.box_id = KOC.global.box_id + 1
    local box = {
        box_id = KOC.global.box_id,
        max_cards = math.random(KOC.global.box_cards),
    }
    return {
        global = KOC.global,
        outputs = {
            { owner = sender(), data = box }
        }
    }
end

function open_box()
    assert(#KOC.inputs == 1 and data(1).box_id, "tiktok: only accept one box")
    local outputs = {}
    -- consume box and mint nft cards
    for _ = 1, data(1).max_cards do
        KOC.global.nft_id = KOC.global.nft_id + 1
        table.insert(outputs, { owner = sender(), data = mint_card(1) })
    end
    return {
        global = KOC.global,
        outputs = outputs
    }
end

function fuse_cards()
    assert(#KOC.inputs > 1, "tiktok: at least 2 cards")
    local baseline = 0
    for _, v in ipairs(KOC.inputs) do
        assert(v.data.id, "tiktok: only accept cards")
        baseline = math.min(baseline + v.data.level * 5, 95)
    end
    KOC.global.nft_id = KOC.global.nft_id + 1
    return {
        global = KOC.global,
        outputs = {
            { owner = sender(), data = mint_card(baseline) }
        }
    }
end

function set_card_program(program)
    assert(#KOC.inputs == 1 and data(1).id, "tiktok: only accept one card")
    assert(type(program) == "string", "tiktok: program must be string")
    local limit = get_card_program_limit(KOC.inputs[1].data.level)
    assert(#program <= limit, "tiktok: program is longer than " .. limit)

    KOC.inputs[1].data.program = program
    return {
        outputs = KOC.inputs
    }
end

function start_tiktok_battle()
    assert(#KOC.inputs == 2 and data(1).id and data(2).id, "tiktok: only accept two cards")
    assert(data(1).program and data(2).program, "tiktok: program needed")

    local tik = data(1)
    local tok = data(2)

    local fn1, err = load(tik.program)
    assert(err == nil, "tiktok: bad program 1")
    local fn2, err = load(tok.program)
    assert(err == nil, "tiktok: bad program 2")

    local ok, pnft1 = pcall(fn1)
    assert(ok, "tiktok: bad program 1")
    local ok, pnft2 = pcall(fn2)
    assert(ok, "tiktok: bad program 2")

    for round = 1, KOC.global.max_rounds do
        pcall(pnft1, round, tik)
        pcall(pnft2, round, tok)
    end

    KOC.global.battle_count = KOC.global.battle_count + 1
    return {
        global = KOC.global
    }
end
