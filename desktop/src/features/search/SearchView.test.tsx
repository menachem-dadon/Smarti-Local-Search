import {render,screen,fireEvent,act,cleanup} from '@testing-library/react';
import {afterEach,beforeEach,describe,it,expect,vi} from 'vitest';
import {SearchView} from './SearchView';
import {en} from '../../i18n/en';
import {search,command,type Settings,type SearchResponse} from '../../app/api';
vi.mock('../../app/api',async original=>({...await original<typeof import('../../app/api')>(),native:()=>false,search:vi.fn(),command:vi.fn().mockResolvedValue(null)}));
const settings={language:'en',default_mode:'exact',search_as_you_type:true,enter_preview:false,debug_logging:false} as Settings;
const response:SearchResponse={id:1,results:[{file:{id:11,root_id:1,path:'C:\\report.txt',name:'report.txt',extension:'txt',kind:'text',size:100,modified:1,created:1,state:'indexed',semantic:true,error:null,online:true},matches:[],score:1,lexical:0,semantic:1,filename:0}],elapsed_ms:1,semantic_ready:true,warning:null};
const t=(key:keyof typeof en)=>en[key];
vi.mock('@tauri-apps/api/core',()=>({convertFileSrc:(path:string)=>path,invoke:vi.fn()}));
async function type(value:string){await act(async()=>{fireEvent.change(screen.getByRole('textbox',{name:'Search'}),{target:{value}})})}
beforeEach(()=>{vi.clearAllMocks();vi.mocked(search).mockResolvedValue(response)});
afterEach(()=>{cleanup();vi.useRealTimers();vi.restoreAllMocks()});
describe('desktop search behavior',()=>{
 it('previews matching media segments and jumps to the selected timestamp',async()=>{
  const matched={id:2,modality:'audio',text:'Matched segment',heading:'',page:null,line_start:null,line_end:null,start:42,end:48};
  const unmatched={...matched,id:3,text:'Unrelated segment',start:90,end:96};
  const file={...response.results[0].file,kind:'audio',name:'tone.wav'};
  vi.mocked(search).mockResolvedValue({...response,results:[{...response.results[0],file,matches:[matched]}]});
  vi.mocked(command).mockImplementation(async action=>action==='get_preview'?{file,chunks:[matched,unmatched],text:'',asset:'tone.wav'}:null);
  const play=vi.spyOn(HTMLMediaElement.prototype,'play').mockResolvedValue();
  const view=render(<SearchView quick settings={settings} status={null} t={t} onError={vi.fn()}/>);
  await type('tone');await act(async()=>{fireEvent.keyDown(window,{key:' '})});
  expect(screen.getByText('Matched segment')).toBeInTheDocument();expect(screen.queryByText('Unrelated segment')).not.toBeInTheDocument();
  fireEvent.click(screen.getByRole('button',{name:/00:00:42/}));
  expect(view.container.querySelector('audio')?.currentTime).toBe(42);expect(play).toHaveBeenCalledOnce();
 });
 it('requires a fresh Enter for each text edit when type search is off',async()=>{
  render(<SearchView settings={{...settings,search_as_you_type:false}} status={null} t={t} onError={vi.fn()}/>);
  await type('alpha');expect(search).not.toHaveBeenCalled();
  await act(async()=>{fireEvent.keyDown(screen.getByRole('textbox'),{key:'Enter'})});expect(search).toHaveBeenCalledTimes(1);
  await type('beta');expect(search).toHaveBeenCalledTimes(1);
  await act(async()=>{fireEvent.keyDown(screen.getByRole('textbox'),{key:'Enter'})});expect(search).toHaveBeenCalledTimes(2);expect(search).toHaveBeenLastCalledWith(expect.objectContaining({query:'beta'}));
 });
 it('does not open hidden search results from another page keyboard event',async()=>{
  const view=render(<SearchView active={false} settings={settings} status={null} t={t} onError={vi.fn()}/>);await type('report');
  fireEvent.keyDown(screen.getByRole('textbox'),{key:'Enter'});expect(command).not.toHaveBeenCalledWith('open_path',expect.anything());
  view.rerender(<SearchView active settings={settings} status={null} t={t} onError={vi.fn()}/>);
  fireEvent.keyDown(screen.getByRole('textbox'),{key:'Enter'});expect(command).toHaveBeenCalledWith('open_path',{id:11});
 });
 it('discards a stale semantic response after newer search input',async()=>{
  vi.useFakeTimers();let finishOld:(response:SearchResponse)=>void=()=>{};
  vi.mocked(search).mockImplementation(async request=>{
    if(request.query==='alpha'&&request.semantic_pass)return new Promise<SearchResponse>(resolve=>{finishOld=resolve});
    return {...response,warning:request.query==='beta'&&request.semantic_pass?'current response':null};
  });
  render(<SearchView settings={{...settings,default_mode:'smart'}} status={null} t={t} onError={vi.fn()}/>);
  await type('alpha');await act(async()=>{vi.advanceTimersByTime(125)});await type('beta');await act(async()=>{vi.advanceTimersByTime(125)});
  expect(screen.getByText('current response')).toBeInTheDocument();
  await act(async()=>{finishOld({...response,warning:'obsolete response'})});expect(screen.queryByText('obsolete response')).not.toBeInTheDocument();expect(screen.getByText('current response')).toBeInTheDocument();
 });
});
