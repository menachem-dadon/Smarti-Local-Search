// Collect a CPU profile only from this task's explicitly launched Vite debugger.
const fs=require('node:fs');
(async()=>{
const endpoints=await(await fetch('http://127.0.0.1:9231/json')).json();
const ws=new WebSocket(endpoints[0].webSocketDebuggerUrl);await new Promise(r=>ws.onopen=r);let next=0;const pending=new Map();
ws.onmessage=e=>{const response=JSON.parse(e.data);if(response.id){pending.get(response.id)?.(response);pending.delete(response.id)}};
const call=(method,params={})=>new Promise(r=>{const id=++next;pending.set(id,r);ws.send(JSON.stringify({id,method,params}))});
await call('Profiler.enable');await call('Profiler.start');await new Promise(r=>setTimeout(r,5000));const {result}=await call('Profiler.stop');ws.close();
fs.writeFileSync('artifacts/vite.cpuprofile',JSON.stringify(result.profile));
const hits=new Map();for(const id of result.profile.samples){hits.set(id,(hits.get(id)||0)+1)}
console.log(result.profile.nodes.map(n=>({hits:hits.get(n.id)||0,...n.callFrame})).sort((a,b)=>b.hits-a.hits).slice(0,15));
})().catch(e=>{console.error(e);process.exitCode=1});
