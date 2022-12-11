
---------------------------
-- contract constants
---------------------------

local MAP_WIDTH = 25
local MAP_HEIGHT = 11
local TIK = 1
local TOK = 2
local TIK_CORE_Y = 1
local TIK_START_Y = 2
local TIK_DIRECTION = 1
local TOK_CORE_Y = MAP_WIDTH
local TOK_START_Y = MAP_WIDTH - 1
local TOK_DIRECTION = -1
local MAX_VIEW_SCOPE = 5

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
            max_rounds = 50,
            max_place_cards = 5
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

local is_array = function (value, len)
    if type(value) == "table" then
        if len then
            return #value == len
        end
        return true
    end
    return false
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

local get_card_move_step = function (level)
    if level <= 3 then
        return 1
    elseif level == 4 then
        return 2
    else
        return 3
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
        view = math.random(MAX_VIEW_SCOPE),
        step = get_card_move_step(level),
        level = level,
    }
end

local search_enemies = function (runner, placement)
    local enemies = {}
    for _, v in ipairs(placement) do
        local x_view = math.abs(v.coordinate.x - runner.coordinate.x)
        local y_view = math.abs(v.coordinate.y - runner.coordinate.y)
        if (x_view + y_view) <= runner.card.view then
            table.insert(enemies, v)
        end
    end
    return enemies
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

function place_tok_cards(coordinates)
    assert(#KOC.inputs <= KOC.global.max_place_cards, "tiktok: exceed max_place_cards")
    assert(is_array(coordinates, #KOC.inputs), "tiktok: mismatched coordinates")

    local placement = {}
    for i, v in ipairs(KOC.inputs) do
        assert(v.data.id and #v.data.program > 0, "tiktok: only accept programmed card")
        assert(coordinates[i].y == TOK_START_Y, "tiktok: invalid tok coordinate")
        table.insert(placement, {
            card = v.data,
            player = sender(),
            coordinate = coordinates[i]
        })
    end
    return {
        outputs = {
            { owner = sender(), data = placement }
        }
    }
end

function unplace_tok_cards()
    assert(#KOC.inputs == 1 and is_array(data(1)), "tiktok: only accept one placement")
    local outputs = {}
    for _, v in ipairs(data(1)) do
        assert(v.coordinate, "tiktok: invalid placement format")
        table.insert(outputs, { owner = sender(), data = v.card })
    end
    return {
        outputs = outputs
    }
end

function start_tiktok_battle(coordinates, battle_id)
    -- prepare battle context
    assert(#KOC.components == 1 and is_array(KOC.components[1]) and KOC.components[1].coordinate,
        "tiktok: invalid tok placement")
    local tok_placement = KOC.components[1]
    assert(#KOC.inputs == coordinates and #KOC.inputs < #tok_placement, "tiktok: exceed tok_placement")
    local tik_placement = {}
    for i, v in ipairs(KOC.inputs) do
        assert(v.data.id and #v.data.program > 0, "tiktok: only accept programmed card")
        assert(coordinates[1].y == TIK_START_Y, "tiktok: invalid tik coordinate")
        table.insert(tik_placement, {
            card = v.data,
            coordinate = coordinates[i]
        })
    end
    local context = {
        width = MAP_WIDTH,
        height = MAP_HEIGHT,
        direction = nil,
        this = nil,
        enemies = nil,
    }
    -- start battle runtime, default with no winner
    local winner = 0;
    math.randomseed(KOC.seeds[1], KOC.seeds[2])
    for i = 1, KOC.global.max_rounds do
        -- run tiktok
        local tiker = tik_placement[math.random(#tik_placement)]
        local tik, err = load(tiker.card.program)
        if err == nil then
            context.direction = TIK_DIRECTION
            context.this = tiker
            context.enemies = search_enemies(tiker, tok_placement)
            local ok, coordinate = pcall(tik, context)
            if ok then
                tiker.coordinate = coordinate
            end
        end
        local toker = tok_placement[math.random(#tok_placement)]
        local tok, err = load(toker.card.program)
        if err == nil then
            context.direction = TOK_DIRECTION
            context.this = toker
            context.enemies = search_enemies(toker, tik_placement)
            local ok, coordinate = pcall(tok, context)
            if ok then
                toker.coordinate = coordinate
            end
        end
        -- check winner
        for _, tiker in ipairs(tik_placement) do
            if tiker.coordinate.y == TOK_CORE_Y then
                winner = TIK
            end
        end
        for _, toker in ipairs(tok_placement) do
            if toker.coordinate.y == TIK_CORE_Y then
                winner = TOK
            end
        end
    end
    -- generate battle receipt
    KOC.global.battle_count = KOC.global.battle_count + 1
    local battle_receipt = {
        battle_id = battle_id,
        winner = winner,
        tik_player = sender(),
        tok_player = tok_placement.player,
        randomseeds = KOC.seeds
    }
    return {
        global = KOC.global,
        outputs = {
            { owner = KOC.owner, data = battle_receipt }
        }
    }
end
