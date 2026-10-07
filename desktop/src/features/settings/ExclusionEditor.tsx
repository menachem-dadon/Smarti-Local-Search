import {useEffect,useRef,useState} from 'react';
import {open} from '@tauri-apps/plugin-dialog';
import {Button,Textarea} from '../../design-system/primitives';
import type {TranslationKey} from '../../i18n/en';

export const exclusionLines=(text:string)=>[...new Map(text.split('\n').map(s=>s.trim()).filter(Boolean).map(s=>[s.replaceAll('\\','/').replace(/\/+$/,'').toLowerCase(),s])).values()];
export function ExclusionEditor({value,onChange,t,onError}:{value:string[];onChange:(value:string[])=>void;t:(k:TranslationKey)=>string;onError:(e:unknown)=>void}){
 const [text,setText]=useState(value.join('\n'));const latest=useRef(text);const [choosing,setChoosing]=useState(false);
 useEffect(()=>{if(JSON.stringify(exclusionLines(latest.current))!==JSON.stringify(value)){latest.current=value.join('\n');setText(latest.current)}},[value]);
 const edit=(next:string)=>{latest.current=next;setText(next);onChange(exclusionLines(next))};
 const choose=async(directory:boolean)=>{
  setChoosing(true);try{const paths=await open({directory,multiple:true,title:t(directory?'excludeFolder':'excludeFile')});if(paths)edit(exclusionLines([latest.current,...(typeof paths==='string'?[paths]:paths)].join('\n')).join('\n'))}catch(e){onError(e)}finally{setChoosing(false)}
 };
 return <div className="exclusion-editor"><Textarea label={t('exclusions')} dir="ltr" rows={8} value={text} onChange={e=>edit(e.target.value)}/><div className="sds-actions"><Button icon="folder" disabled={choosing} onClick={()=>void choose(true)}>{t('excludeFolder')}</Button><Button icon="text" disabled={choosing} onClick={()=>void choose(false)}>{t('excludeFile')}</Button></div><p className="sds-hint">{t('exclusionHint')}</p></div>;
}
