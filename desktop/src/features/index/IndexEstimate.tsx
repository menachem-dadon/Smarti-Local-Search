import {type Status} from '../../app/api';
import type {TranslationKey} from '../../i18n/en';
export function duration(seconds:number,language:string){
 if(seconds<60)return new Intl.NumberFormat(language,{style:'unit',unit:'second',unitDisplay:'long'}).format(Math.max(1,Math.ceil(seconds)));
 const minutes=Math.max(1,Math.ceil(seconds/60));
 const units=minutes>=1440?[[Math.floor(minutes/1440),'day'],[Math.floor(minutes%1440/60),'hour']]:minutes>=60?[[Math.floor(minutes/60),'hour'],[minutes%60,'minute']]:[[minutes,'minute']];
 return units.filter(([n])=>Number(n)>0).map(([n,unit])=>new Intl.NumberFormat(language,{style:'unit',unit:String(unit),unitDisplay:'long'}).format(Number(n))).join(' · ');
}
export function IndexEstimate({status,t,language}:{status:Status|null;t:(k:TranslationKey)=>string;language:string}){
 if(!status)return null;
 const discovery=status.discovery_complete?t('complete'):!status.running||status.paused?t('estimatePaused'):status.discovery_counting||status.discovery_eta_seconds==null?t('estimating'):duration(status.discovery_eta_seconds,language);
 const indexing=status.running&&!status.discovery_complete?t('afterDiscovery'):!status.pending?t('complete'):!status.running||status.paused?t('estimatePaused'):status.index_eta_seconds==null?t('estimating'):duration(status.index_eta_seconds,language);
 return <div className="index-estimates" aria-live="polite"><dl className="index-info"><div><dt>{t('discoveryRemaining')}</dt><dd>{discovery}</dd>{!status.discovery_counting&&!!status.discovery_total&&<small><bdi>{status.discovery_processed?.toLocaleString()} / {status.discovery_total.toLocaleString()}</bdi></small>}</div><div><dt>{t('indexRemaining')}</dt><dd>{indexing}</dd></div></dl>{status.running&&!status.paused&&<p className="sds-hint">{t(status.index_eta_provisional?'estimateProvisional':'estimateHint')}</p>}</div>;
}
