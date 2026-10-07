import {invoke} from '@tauri-apps/api/core';
export interface Settings { language:string;theme:string;reduced_motion:boolean;text_size:number;onboarded:boolean;default_mode:string;results_count:number;search_as_you_type:boolean;enter_preview:boolean;show_offline:boolean;profile:string;accelerator:string;threads:number;concurrency:number;dimensions:number;vision_tokens:number;text:boolean;code:boolean;documents:boolean;images:boolean;audio:boolean;video:boolean;audio_seconds:number;video_seconds:number;video_fps:number;pause_on_battery:boolean;network_drives:boolean;sensitive_files:boolean;exclusions:string[];max_text_mb:number;cache_mb:number;keep_ready:boolean;minimize_to_tray:boolean;autostart:boolean;quick_search:boolean;shortcut:string;explorer_menu:boolean;notifications:boolean;debug_logging:boolean;index_path:string; }
export interface Root {id:number;path:string;online:boolean;files:number;indexed:number;exclusions:string[]}
export interface FileRecord {id:number;root_id:number;path:string;name:string;extension:string;kind:string;size:number;modified:number;created:number;state:string;semantic:boolean;error:string|null;online:boolean}
export interface Chunk {id:number;modality:string;text:string;heading:string;page:number|null;line_start:number|null;line_end:number|null;start:number|null;end:number|null}
export interface Result {file:FileRecord;matches:Chunk[];score:number;lexical:number;semantic:number;filename:number}
export interface SearchResponse {id:number;results:Result[];elapsed_ms:number;semantic_ready:boolean;warning:string|null}
export interface Preview {file:FileRecord;chunks:Chunk[];text:string;asset:string|null}
export interface Status {stage:string;running:boolean;paused:boolean;discovered:number;files:number;semantic_files:number;chunks:number;vectors:number;errors:number;pending:number;bytes:number;files_per_second:number;embeddings_per_second:number;updated:number;inference:{ready?:boolean;state?:string;error?:string;backend?:string;dimensions?:number;probes?:unknown[];timings?:Record<string,number>};watcher:string}
export interface Activity {id:number;file_id:number|null;time:number;kind:string;path:string;message:string}
export const native=()=>Boolean('__TAURI_INTERNALS__' in window);
export async function command<T=unknown>(action:string,args:Record<string,unknown>={}):Promise<T>{if(!native())throw new Error('Native desktop service unavailable');return invoke<T>('command',{action,args});}
export async function search(request:Record<string,unknown>){return invoke<SearchResponse>('search',{request});}
export function bytes(n:number){if(n<1024)return `${n} B`;const e=Math.min(Math.floor(Math.log(n)/Math.log(1024)),3);return `${(n/1024**e).toFixed(1)} ${['B','KB','MB','GB'][e]}`;}
export function timestamp(seconds:number){return `${Math.floor(seconds/3600).toString().padStart(2,'0')}:${Math.floor(seconds/60%60).toString().padStart(2,'0')}:${Math.floor(seconds%60).toString().padStart(2,'0')}`;}
