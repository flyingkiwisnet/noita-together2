export interface RoomSemantics {revision:string;kind:string;requiredCapabilities:string[]}
export const ROOM_SEMANTICS_REVISION:string;
export function validateRoomSemantics(value:unknown,compatibility:{capabilities:string[]}):string[];
export function declareRoomSemantics(compatibility:{capabilities:string[]}):RoomSemantics;
