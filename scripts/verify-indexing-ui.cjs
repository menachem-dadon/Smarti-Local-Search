// Browser evidence only: the actual React UI with an in-memory native IPC
// adapter. Native Windows dialogs and the installed app require separate QA.
const {chromium}=require('playwright');
const fs=require('node:fs');
const path=require('node:path');
const assert=require('node:assert/strict');
async function main(){
 const output=path.resolve(process.argv[3]||'artifacts/indexing-ui');fs.mkdirSync(output,{recursive:true});
 const browser=await chromium.launch({headless:true,channel:process.platform==='win32'?'msedge':undefined});
 try{
  const page=await browser.newPage({viewport:{width:1280,height:800}});
  const errors=[];page.on('pageerror',e=>{errors.push(e.message);console.error('Browser error:',e.message)});
  await page.addInitScript(()=>{
   let next=1;const callbacks=new Map(),listeners=new Map();window.qaCalls=[];
   let settings={language:'he',theme:'system',reduced_motion:false,text_size:16,onboarded:true,default_mode:'smart',results_count:100,search_as_you_type:true,enter_preview:false,show_offline:false,profile:'auto',accelerator:'auto',threads:4,concurrency:1,dimensions:256,vision_tokens:70,text:true,code:true,documents:true,images:true,audio:true,video:true,audio_seconds:60,video_seconds:30,video_fps:1,pause_on_battery:true,network_drives:false,sensitive_files:false,exclusions:['node_modules','.cache','obj'],max_text_mb:32,cache_mb:512,keep_ready:true,minimize_to_tray:true,autostart:false,quick_search:true,shortcut:'Ctrl+Alt+Space',explorer_menu:false,notifications:true,debug_logging:false,index_path:''};
   const status={stage:'discovery',running:true,paused:false,discovered:2500,files:2500,semantic_files:0,chunks:0,vectors:0,errors:0,pending:2500,bytes:4000000,files_per_second:500,embeddings_per_second:0,discovery_total:12000,discovery_processed:2500,discovery_counting:false,discovery_complete:false,discovery_eta_seconds:19,index_eta_seconds:null,index_eta_provisional:true,updated:0,inference:{ready:true,backend:'cpu'},watcher:'active'};
   window.__TAURI_EVENT_PLUGIN_INTERNALS__={unregisterListener:()=>{}};
   window.__TAURI_INTERNALS__={metadata:{currentWindow:{label:'main'},currentWebview:{label:'main'}},transformCallback:fn=>{const id=next++;callbacks.set(id,fn);return id},unregisterCallback:id=>callbacks.delete(id),invoke:async(cmd,args)=>{
    if(cmd==='plugin:event|listen'){listeners.set(args.event,args.handler);return next++}
    if(cmd==='plugin:event|unlisten')return;
    if(cmd==='plugin:dialog|open'){window.qaCalls.push({cmd,args});return args.options.directory?['C:\\QA\\Cache']:['C:\\QA\\private.txt']}
    if(cmd==='search')return {id:args.request.id,results:[],elapsed_ms:1,semantic_ready:true,warning:null};
    if(cmd!=='command')return null;
    const {action,args:params}=args;
    if(action==='get_settings')return settings;
    if(action==='get_status')return status;
    if(action==='get_roots')return [{id:1,path:'C:\\QA',online:true,files:2500,indexed:0,exclusions:[]}];
    if(action==='get_activity')return [];
    if(action==='initial_input')return {query:''};
    if(action==='update_settings'){settings=params.settings;window.qaCalls.push({action,settings});const callback=callbacks.get(listeners.get('settings-changed'));if(callback)callback({event:'settings-changed',id:1,payload:settings});return settings}
    if(action==='storage_info')return {path:'C:\\QA\\Index'};
    return null;
   }};
  });
  await page.goto(process.argv[2]||'http://127.0.0.1:1435/');
  const input=page.locator('#main-search');await input.waitFor();await input.focus();
  const styles=await input.evaluate(e=>({outline:getComputedStyle(e).outlineStyle,align:getComputedStyle(e).textAlign,placeholder:getComputedStyle(e,'::placeholder').textAlign,outer:getComputedStyle(e.closest('.search-box')).outlineStyle}));
  assert.equal(styles.outline,'none');assert.equal(styles.align,'right');assert.equal(styles.placeholder,'right');assert.equal(styles.outer,'solid');
  await page.screenshot({path:path.join(output,'search-focus.png')});
  await page.getByRole('button',{name:'אינדקס',exact:true}).click();
  await page.getByText('גילוי קבצים: זמן משוער שנותר').waitFor();
  assert.equal(await page.getByText('2,500 / 12,000').count(),1);
  await page.screenshot({path:path.join(output,'index-estimates.png')});
  await page.getByRole('button',{name:'הגדרות',exact:true}).click();
  assert.equal(await page.getByRole('button',{name:'שמירה',exact:true}).count(),0);
  await page.getByRole('button',{name:'החרגות',exact:true}).click();
  await page.getByRole('button',{name:'החרגת תיקייה',exact:true}).click();
  await page.waitForFunction(()=>window.qaCalls.some(c=>c.action==='update_settings'&&c.settings.exclusions.includes('C:\\QA\\Cache')));
  await page.getByRole('button',{name:'החרגת קובץ',exact:true}).click();
  await page.waitForFunction(()=>window.qaCalls.some(c=>c.action==='update_settings'&&c.settings.exclusions.includes('C:\\QA\\private.txt')));
  await page.screenshot({path:path.join(output,'settings-exclusions.png')});
  await page.getByRole('button',{name:'מיקומים',exact:true}).click();
  await page.getByRole('button',{name:'פעולות נוספות',exact:true}).click();
  assert.equal(await page.getByRole('menuitem',{name:'החרגות',exact:true}).count(),0);
  await page.keyboard.press('Escape');
  await page.getByRole('button',{name:'הגדרות',exact:true}).click();
  await page.getByRole('button',{name:'החרגות',exact:true}).click();
  await page.setViewportSize({width:760,height:800});
  const overflow=await page.evaluate(()=>document.documentElement.scrollWidth>innerWidth);
  assert.equal(overflow,false);assert.deepEqual(errors,[]);
  const report={scope:'Actual React in Chromium with mocked native IPC; no installed/native dialog claim',styles,overflow,errors,saves:await page.evaluate(()=>window.qaCalls.filter(c=>c.action==='update_settings').length)};
  fs.writeFileSync(path.join(output,'report.json'),JSON.stringify(report,null,2));console.log(JSON.stringify(report));
 }finally{await browser.close()}
}
main().catch(e=>{console.error(e);process.exitCode=1});
