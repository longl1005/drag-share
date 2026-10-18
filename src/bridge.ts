import {invoke,isTauri} from '@tauri-apps/api/core';
import type {DesktopBridge,Outcome,Snapshot} from './types';
export const native=isTauri();
export const bridge:DesktopBridge={
 snapshot:()=>invoke<Snapshot>('snapshot'),setEnabled:enabled=>invoke<Outcome>('set_enabled',{enabled}),
 chooseDirectory:()=>invoke<string|null>('choose_directory'),add:path=>invoke<Outcome>('add_share',{path}),
 remove:id=>invoke<Outcome>('remove_share',{id}),copy:id=>invoke('copy_address',{id:id??null}),
 reveal:id=>invoke('reveal_share',{id}),settings:()=>invoke('open_settings'),hide:()=>invoke('hide_panel'),
};
