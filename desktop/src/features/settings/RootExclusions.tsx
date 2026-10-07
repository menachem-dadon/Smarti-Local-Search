import {useEffect,useRef,useState} from 'react';
import {command,type Root} from '../../app/api';
import type {TranslationKey} from '../../i18n/en';
import {ExclusionEditor} from './ExclusionEditor';
export function RootExclusions({root,t,onSaved,onError}:{root:Root;t:(k:TranslationKey)=>string;onSaved:()=>void;onError:(e:unknown)=>void}){
 const [value,setValue]=useState(root.exclusions),latest=useRef(root.exclusions),saved=useRef(root.exclusions),saving=useRef(false),timer=useRef<ReturnType<typeof setTimeout>>(undefined);
 const callbacks=useRef({onSaved,onError});callbacks.current={onSaved,onError};
 const persist=async()=>{
  clearTimeout(timer.current);if(saving.current||JSON.stringify(latest.current)===JSON.stringify(saved.current))return;
  saving.current=true;const sent=latest.current;let succeeded=false;
  try{await command('update_root',{id:root.id,exclusions:sent});saved.current=sent;callbacks.current.onSaved();succeeded=true}catch(e){callbacks.current.onError(e)}
  finally{saving.current=false;if(succeeded&&JSON.stringify(latest.current)!==JSON.stringify(saved.current))void persist()}
 };
 useEffect(()=>()=>{void persist()},[]);
 return <ExclusionEditor value={value} t={t} onError={onError} onChange={v=>{latest.current=v;setValue(v);clearTimeout(timer.current);timer.current=setTimeout(()=>void persist(),450)}}/>;
}
