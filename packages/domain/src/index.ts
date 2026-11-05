import { BoundaryError, type Command, type Room, type Session, limits } from '@nt2/contracts';

// Deliberately limited mock scenario rules. D06/D09 production semantics remain open.
export function applyMockCommand(room: Room, session: Session, command: Command, displayName: string, newRunId: () => string): void {
  if (command.connectionEpoch !== session.epoch) throw new BoundaryError('STALE_EPOCH');
  if (command.expectedRevision !== room.revision) throw new BoundaryError('STALE_REVISION');
  if (command.runId !== room.runId) throw new BoundaryError('WRONG_RUN');
  if (command.kind === 'transfer.status') throw new BoundaryError('UNSUPPORTED_G0');
  const player = room.players.find(p => p.connectionId === (session.attachedDesktopId ?? session.connectionId));
  if (command.kind === 'room.join') {
    if (!['game','desktop'].includes(session.kind) || session.attachedDesktopId) throw new BoundaryError('FORBIDDEN');
    if (room.state !== 'waiting') throw new BoundaryError('CONFLICT');
    if (player) return;
    if (room.players.length >= room.capacity) throw new BoundaryError('ROOM_FULL');
    room.players.push({ accountId: session.accountId, connectionId: session.connectionId, epoch: session.epoch, displayName, ready: false, seedAccepted: false, loaded: false });
  } else if (command.kind === 'run.start' || command.kind === 'run.cancel') {
    if (room.hostAccountId !== session.accountId) throw new BoundaryError('FORBIDDEN');
    if (command.kind === 'run.start') {
      if (room.state !== 'waiting') throw new BoundaryError('CONFLICT');
      if (!room.players.length || room.players.some(p => !p.ready)) throw new BoundaryError('NOT_READY');
      room.roster = room.players.map(p => p.connectionId);
      room.runId = newRunId(); room.state = 'preparing';
    } else {
      if (!['preparing','loading'].includes(room.state)) throw new BoundaryError('CONFLICT');
      room.state = 'cancelled';
    }
  } else {
    const browserControl = session.kind === 'browser' && session.accountId === room.hostAccountId;
    if (!player && !(command.kind === 'chat.send' && browserControl)) throw new BoundaryError('FORBIDDEN');
    if (command.kind === 'chat.send') {
      room.chat.push({accountId: session.accountId, text: command.body.text as string});
      room.chat = room.chat.slice(-limits.chatHistory);
    } else if (command.kind === 'room.leave') {
      if (room.state !== 'waiting') throw new BoundaryError('UNSUPPORTED_G0');
      room.players = room.players.filter(p => p !== player);
    } else if (command.kind === 'game.ready') {
      if (session.attachedDesktopId) throw new BoundaryError('FORBIDDEN');
      if (room.state !== 'waiting') throw new BoundaryError('CONFLICT');
      player!.ready = command.body.ready as boolean;
    } else if (command.kind === 'game.seedAccepted') {
      if (session.kind !== 'game') throw new BoundaryError('FORBIDDEN');
      if (room.state !== 'preparing' || command.body.seed !== room.seed) throw new BoundaryError('CONFLICT');
      player!.seedAccepted = true;
      if (room.players.every(p => p.seedAccepted)) room.state = 'loading';
    } else if (command.kind === 'game.loaded') {
      if (session.kind !== 'game') throw new BoundaryError('FORBIDDEN');
      if (room.state !== 'loading' || !player!.seedAccepted) throw new BoundaryError('CONFLICT');
      player!.loaded = true;
      if (room.players.every(p => p.loaded)) room.state = 'running';
    }
  }
  room.revision++;
}
