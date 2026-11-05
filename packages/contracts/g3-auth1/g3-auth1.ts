// GENERATED from schema.json. Do not edit.
export const AUTH1_LIMITS = {"bytes":65536,"fields":2048,"depth":8} as const;
export interface Provider {
  id?: string;
  available?: boolean;
  reason?: string;
}
export interface Discovery {
  revision?: string;
  serviceId?: string;
  serviceEpoch?: string;
  providers?: Provider[];
  signInSeconds?: number;
  accountEndpoint?: string;
  passwordChangeUrl?: string;
}
export interface Entitlements {
  customLobbyName?: boolean;
  capacityLimit?: number;
  standardCapacityLimit?: number;
  powerCapacityLimit?: number;
  generatedLobbyName?: string;
  testingMode?: boolean;
  manageUsers?: boolean;
  manageAllRoles?: boolean;
}
export interface Account {
  accountId?: string;
  displayName?: string;
  provider?: string;
  role?: string;
  enabled?: boolean;
  revision?: number;
  lastSignInAt?: string;
  entitlements?: Entitlements;
}
export interface Error {
  code?: string;
}
export interface AccountResponse {
  revision?: string;
  serviceId?: string;
  serviceEpoch?: string;
  account?: Account;
  error?: Error;
}
export interface Empty {
}
export interface RoleChange {
  accountId?: string;
  role?: string;
  revision?: number;
  operationId?: string;
  reason?: string;
}
export interface LocalCreate {
  username?: string;
  displayName?: string;
  operationId?: string;
}
export interface LocalReset {
  accountId?: string;
  revision?: number;
  operationId?: string;
  reason?: string;
}
