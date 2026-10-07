import {useEffect,useRef,useState} from 'react';
import {command,type Settings} from '../../app/api';

const semanticKeys=['dimensions','vision_tokens','audio_seconds','video_seconds','video_fps'] as const;
const same=(a:unknown,b:unknown)=>JSON.stringify(a)===JSON.stringify(b);
const valid=(s:Settings)=>[
 [s.results_count,1,500],[s.threads,1,128],[s.text_size,13,24],
 [s.max_text_mb,1,512],[s.cache_mb,32,8192],[s.audio_seconds,10,120],
 [s.video_seconds,10,60],[s.video_fps,0.1,4],[s.vision_tokens,1,1120],
].every(([v,min,max])=>Number.isFinite(v)&&v>=min&&v<=max)
 &&[s.results_count,s.threads,s.text_size,s.max_text_mb,s.cache_mb,s.audio_seconds,s.video_seconds,s.vision_tokens].every(Number.isInteger)
 &&Math.abs(s.video_fps*10-Math.round(s.video_fps*10))<1e-6
 &&[128,256,512,768].includes(s.dimensions)&&Boolean(s.shortcut.trim());

export function useSettingsAutosave(settings:Settings,onSave:(s:Settings)=>void,onError:(e:unknown)=>void){
 const [draft,setDraft]=useState(settings),[busy,setBusy]=useState(false),[error,setError]=useState<string>(),[semanticPending,setSemanticPending]=useState(false);
 const current=useRef(settings),baseline=useRef(settings),dirty=useRef(new Set<keyof Settings>()),inflight=useRef(false),timer=useRef<ReturnType<typeof setTimeout>>(undefined),mounted=useRef(true);
 const callbacks=useRef({onSave,onError});callbacks.current={onSave,onError};
 const completion=useRef<Promise<void>>(Promise.resolve());
 const publish=()=>{if(mounted.current)setDraft({...current.current})};
 const receive=(saved:Settings)=>{
  baseline.current=saved;
  for(const key of [...dirty.current])if(same(current.current[key],saved[key]))dirty.current.delete(key);
  current.current={...saved,...Object.fromEntries([...dirty.current].map(k=>[k,current.current[k]]))};publish();
 };
 useEffect(()=>{receive(settings)},[settings]);
 const schedule=()=>{clearTimeout(timer.current);timer.current=setTimeout(()=>{if(mounted.current&&valid(current.current))setSemanticPending(semanticKeys.some(k=>dirty.current.has(k)));void flush()},450)};
 const flush=async(rebuild=false)=>{
  clearTimeout(timer.current);
  if(inflight.current){await completion.current;return flush(rebuild)}
  const keys=[...dirty.current].filter(k=>rebuild||!semanticKeys.includes(k as typeof semanticKeys[number]));
  if(!keys.length)return;
  const sent={...baseline.current,...Object.fromEntries(keys.map(k=>[k,current.current[k]]))};
  if(!valid(sent))return;
  inflight.current=true;if(mounted.current){setBusy(true);setError(undefined)}
  let finish=()=>{};completion.current=new Promise<void>(resolve=>{finish=resolve});
  let succeeded=false;
  try{
   if(rebuild)await command('index_control',{control:'stop'});
   const saved=await command<Settings>('update_settings',{settings:sent,rebuild});
   receive(saved);callbacks.current.onSave(saved);succeeded=true;
   if(rebuild&&mounted.current)setSemanticPending(semanticKeys.some(k=>dirty.current.has(k)));
  }catch(e){if(mounted.current)setError(String(e));callbacks.current.onError(e)}
  finally{inflight.current=false;finish();if(mounted.current)setBusy(false);if(succeeded&&dirty.current.size)schedule()}
 };
 // Leaving Settings immediately after an edit still commits that edit. Semantic
 // changes remain uncommitted until the user confirms the existing rebuild flow.
 useEffect(()=>{mounted.current=true;return()=>{mounted.current=false;void flush()}},[]);
 const update=<K extends keyof Settings>(key:K,value:Settings[K])=>{
  current.current={...current.current,[key]:value};
  if(same(value,baseline.current[key]))dirty.current.delete(key);else dirty.current.add(key);
  publish();setError(undefined);
  schedule();
 };
 const cancelSemantic=()=>{
  for(const key of semanticKeys){current.current={...current.current,[key]:baseline.current[key]};dirty.current.delete(key)}
  setSemanticPending(false);publish();
 };
 const replace=(saved:Settings)=>{clearTimeout(timer.current);dirty.current.clear();receive(saved);callbacks.current.onSave(saved);if(mounted.current)setSemanticPending(false)};
 const reset=async()=>{await flush();clearTimeout(timer.current);replace(await command<Settings>('reset_settings'))};
 return {draft,update,busy,error,semanticPending,cancelSemantic,confirmSemantic:()=>flush(true),replace,reset};
}
