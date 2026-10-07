import {act,cleanup,fireEvent,render,screen} from '@testing-library/react';
import {afterEach,beforeEach,describe,expect,it,vi} from 'vitest';
import {SettingsView} from './SettingsView';
import {ExclusionEditor} from './ExclusionEditor';
import {command,type Settings} from '../../app/api';
import {open} from '@tauri-apps/plugin-dialog';
import {en} from '../../i18n/en';
import {DesignSystemProvider} from '../../design-system/primitives';
vi.mock('../../app/api',async original=>({...await original<typeof import('../../app/api')>(),command:vi.fn()}));
vi.mock('@tauri-apps/plugin-dialog',()=>({open:vi.fn()}));
const settings={language:'en',theme:'system',exclusions:['node_modules'],shortcut:'Ctrl+Alt+Space',results_count:100,threads:4,text_size:16,max_text_mb:32,cache_mb:512,audio_seconds:60,video_seconds:30,video_fps:1,vision_tokens:70,dimensions:256,default_mode:'smart',profile:'auto',accelerator:'auto',quick_search:true,minimize_to_tray:true} as Settings;
const t=(key:keyof typeof en)=>en[key];
const mount=()=>render(<DesignSystemProvider theme="light" dir="ltr"><SettingsView settings={settings} status={null} t={t} onSave={vi.fn()} onError={vi.fn()}/></DesignSystemProvider>);
const tick=()=>act(async()=>{await vi.advanceTimersByTimeAsync(500)});
const updates=()=>vi.mocked(command).mock.calls.filter(([action])=>action==='update_settings');
beforeEach(()=>{vi.useFakeTimers();vi.clearAllMocks();vi.mocked(command).mockImplementation(async(action,args)=>action==='update_settings'?args?.settings:action==='storage_info'?{path:'C:\\TestIndex'}:null)});
afterEach(async()=>{cleanup();await act(async()=>{await vi.runOnlyPendingTimersAsync()});vi.useRealTimers()});
describe('automatic settings persistence',()=>{
 it('keeps Windows settings in one category and saves without a save button',async()=>{
  mount();expect(screen.queryByRole('switch',{name:en.autostart})).not.toBeInTheDocument();
  fireEvent.click(screen.getByRole('button',{name:en.windows}));
  expect(screen.getAllByRole('switch',{name:en.autostart})).toHaveLength(1);
  fireEvent.click(screen.getByRole('switch',{name:en.autostart}));await tick();
  expect(updates()).toHaveLength(1);expect(updates()[0][1]).toMatchObject({settings:{autostart:true}});
  expect(screen.queryByRole('button',{name:en.save})).not.toBeInTheDocument();
 });
 it('preserves a newer draft during an in-flight save and an incoming settings event',async()=>{
  let resolve:(s:Settings)=>void=()=>{};
  vi.mocked(command).mockImplementationOnce(async()=>new Promise<Settings>(r=>{resolve=r}));
  const view=mount();fireEvent.click(screen.getByRole('button',{name:en.windows}));
  fireEvent.change(screen.getByLabelText(en.shortcut),{target:{value:'Ctrl+Shift+K'}});await tick();
  fireEvent.change(screen.getByLabelText(en.shortcut),{target:{value:'Ctrl+Shift+L'}});
  const acknowledged={...settings,shortcut:'Ctrl+Shift+K'};
  view.rerender(<DesignSystemProvider theme="light" dir="ltr"><SettingsView settings={acknowledged} status={null} t={t} onSave={vi.fn()} onError={vi.fn()}/></DesignSystemProvider>);
  await act(async()=>{resolve(acknowledged)});await tick();
  expect(screen.getByLabelText(en.shortcut)).toHaveValue('Ctrl+Shift+L');
  expect(updates().at(-1)?.[1]).toMatchObject({settings:{shortcut:'Ctrl+Shift+L'}});
 });
 it('commits when leaving settings immediately after editing',async()=>{
  const view=mount();fireEvent.click(screen.getByRole('button',{name:en.windows}));
  fireEvent.click(screen.getByRole('switch',{name:en.autostart}));
  await act(async()=>view.unmount());expect(updates()[0][1]).toMatchObject({settings:{autostart:true}});
 });
 it('keeps invalid numeric drafts out of persistence',async()=>{
  mount();fireEvent.click(screen.getByRole('button',{name:en.search}));
  fireEvent.change(screen.getByLabelText(en.count),{target:{value:''}});await tick();expect(updates()).toHaveLength(0);
  fireEvent.change(screen.getByLabelText(en.count),{target:{value:'20.5'}});await tick();expect(updates()).toHaveLength(0);
  fireEvent.change(screen.getByLabelText(en.count),{target:{value:'120'}});await tick();expect(updates()[0][1]).toMatchObject({settings:{results_count:120}});
 });
 it('requires semantic rebuild confirmation and restores cancelled values',async()=>{
  mount();fireEvent.click(screen.getByRole('button',{name:en.storage}));
  fireEvent.change(screen.getByLabelText(en.dimensions),{target:{value:'128'}});await tick();
  expect(updates()).toHaveLength(0);expect(screen.getByRole('alertdialog')).toBeInTheDocument();
  fireEvent.click(screen.getByRole('button',{name:en.cancel}));await tick();expect(screen.getByLabelText(en.dimensions)).toHaveValue('256');
  fireEvent.change(screen.getByLabelText(en.dimensions),{target:{value:'128'}});await tick();
  await act(async()=>{fireEvent.click(screen.getByRole('button',{name:en.confirm}))});
  expect(command).toHaveBeenCalledWith('index_control',{control:'stop'});
  expect(updates()[0][1]).toMatchObject({settings:{dimensions:128},rebuild:true});
 });
 it('retains the draft and exposes persistence failures',async()=>{
  vi.mocked(command).mockRejectedValueOnce(new Error('Persistence failed'));
  mount();fireEvent.click(screen.getByRole('button',{name:en.windows}));fireEvent.click(screen.getByRole('switch',{name:en.autostart}));await tick();
  expect(screen.getByText('Error: Persistence failed')).toBeInTheDocument();expect(screen.getByRole('switch',{name:en.autostart})).toBeChecked();
 });
});
describe('native exclusion chooser',()=>{
 it('keeps folder names and full paths together in one global settings category',async()=>{
  mount();fireEvent.click(screen.getByRole('button',{name:en.index}));
  expect(screen.queryByLabelText(en.exclusions)).not.toBeInTheDocument();
  fireEvent.click(screen.getByRole('button',{name:en.exclusions}));
  expect(screen.getAllByRole('textbox',{name:en.exclusions})).toHaveLength(1);
  fireEvent.change(screen.getByRole('textbox',{name:en.exclusions}),{target:{value:'node_modules\nC:\\Private'}});await tick();
  expect(updates()[0][1]).toMatchObject({settings:{exclusions:['node_modules','C:\\Private']}});
 });
 it('preserves line editing and appends full folder/file paths from native dialogs',async()=>{
  const changed=vi.fn();render(<ExclusionEditor value={['node_modules']} onChange={changed} t={t} onError={vi.fn()}/>);
  fireEvent.change(screen.getByLabelText(en.exclusions),{target:{value:'node_modules\n'}});
  expect(screen.getByLabelText(en.exclusions)).toHaveValue('node_modules\n');
  vi.mocked(open).mockResolvedValueOnce(['C:\\Cache','C:\\Cache']);
  await act(async()=>{fireEvent.click(screen.getByRole('button',{name:en.excludeFolder}))});
  expect(open).toHaveBeenCalledWith(expect.objectContaining({directory:true,multiple:true}));
  expect(changed).toHaveBeenLastCalledWith(['node_modules','C:\\Cache']);
  vi.mocked(open).mockResolvedValueOnce('C:\\Documents\\secret.txt');
  await act(async()=>{fireEvent.click(screen.getByRole('button',{name:en.excludeFile}))});
  expect(open).toHaveBeenLastCalledWith(expect.objectContaining({directory:false}));
  expect(changed).toHaveBeenLastCalledWith(['node_modules','C:\\Cache','C:\\Documents\\secret.txt']);
 });
});
