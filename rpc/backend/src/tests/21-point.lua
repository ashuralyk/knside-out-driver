
function construct ()
    return {
        battle_count = 0,
        nft_token_id = 0,
        users = {}
    }
end

function battle_win()
    local global = msg.global
    global.battle_count = global.battle_count + 1
    local user = global.users[msg.sender] or {
        win_count = 0,
        lose_count = 0,
        nfts = {}
    }
    user.win_count = user.win_count + 1
    table.insert(user.nfts, global.nft_token_id)
    global.nft_token_id = global.nft_token_id + 1 
    global.users[msg.sender] = user
    return {
        owner = msg.sender,
        data = nil
    }
end

function battle_lose()
    local global = msg.global
    global.battle_count = global.battle_count + 1
    local user = global.users[msg.sender] or {
        win_count = 0,
        lose_count = 0,
        nfts = {}
    }
    user.lose_count = user.lose_count + 1
    global.users[msg.sender] = user
    return {
        owner = msg.sender,
        data = nil
    }
end

function claim_nfts()
    local user = msg.global.users[msg.sender]
    assert(user and #user.nfts > 0, "no user or nfts")
    local reward_nfts = user.nfts
    user.nfts = {}
    if #msg.data > 0 then
        for _, nft_id in ipairs(reward_nfts) do
            table.insert(msg.data, nft_id)
        end
    else
        msg.data = reward_nfts
    end
    return {
        owner = msg.sender,
        data = msg.data
    }
end
