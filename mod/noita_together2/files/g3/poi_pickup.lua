-- Local-only marker removal. Owner detects it and retains private dismissal before ack.
function item_pickup(entity_item)
 EntityKill(entity_item)
end
