export type ServiceStatus = 'enabled' | 'disabled' | 'unknown';
export interface ShareItem { id:string; name:string; path:string; readOnly:boolean; guestAccess:boolean; exists:boolean }
export interface Snapshot { status:ServiceStatus; shares:ShareItem[]; address:string; addressFallback:boolean; warnings:string[] }
export interface Outcome { canceled:boolean }
export interface AppError { code:string; message:string }
export interface DesktopBridge {
  snapshot():Promise<Snapshot>; setEnabled(enabled:boolean):Promise<Outcome>; chooseDirectory():Promise<string|null>;
  add(path:string):Promise<Outcome>; remove(id:string):Promise<Outcome>; copy(id?:string):Promise<void>;
  reveal(id:string):Promise<void>; settings():Promise<void>; hide():Promise<void>;
}
