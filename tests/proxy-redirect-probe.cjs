const http=require('node:http');
const {resolve}=require('node:path');
async function probe(root){
 let destinationCalls=0;
 const calls=[];
 const target=http.createServer((req,res)=>{destinationCalls++;res.writeHead(200,{'content-type':'application/json'});res.end(JSON.stringify({destination:'redirect-marker',method:req.method}));});
 await new Promise(r=>target.listen(0,'127.0.0.1',r));
 const destination='http://127.0.0.1:'+target.address().port+'/unexpected';
 const source=http.createServer(async(req,res)=>{
  let raw='';for await(const c of req)raw+=c;
  const {status}=JSON.parse(raw);
  calls.push({method:req.method,path:req.url,correlation:req.headers['x-correlation-id']});
  res.writeHead(status,{'content-type':'application/json',...( [301,302,303,307,308].includes(status)?{location:destination}:{})});
  res.end(JSON.stringify({status,engine:true}));
 });
 await new Promise(r=>source.listen(0,'127.0.0.1',r));
 process.env.ORCHESTRATOR_ENGINE_URL='http://127.0.0.1:'+source.address().port;
 process.env.ORCHESTRATOR_ENGINE_SKIP_AUTH='true';
 process.env.ORCHESTRATOR_ENGINE_TIMEOUT_MS='2000';
 const {handler}=require(resolve(root,'functions/index.js'));
 async function check(status,redirect){
  const before=destinationCalls,headers={};let actual,body;
  await handler({method:'POST',path:'/v1/orchestrator/workflow',headers:{'x-correlation-id':'redirect-fixture'},body:{status}},{
   set(k,v){headers[k]=v;return this;},status(v){actual=v;return this;},json(v){body=v;return this;},send(v){body=v;return this;}
  });
  const last=calls.at(-1),forwarded=last?.method==='POST'&&last.path==='/v1/orchestrator/workflow'&&last.correlation==='redirect-fixture'&&headers['Access-Control-Allow-Origin']==='*';
  const targetRequests=destinationCalls-before;
  const pass=redirect?
   actual===502&&/redirect/i.test(body?.error)&&body.upstream_status===status&&body.execution_metadata?.service==='orchestrator-agents'&&body.layers_executed?.[1]?.status==='error'&&targetRequests===0&&!JSON.stringify(body).includes('redirect-marker')&&!JSON.stringify(body).includes('/unexpected'):
   actual===status&&JSON.stringify(body)===JSON.stringify(status===304?{}:{status,engine:true})&&headers['X-Correlation-ID']==='redirect-fixture'&&targetRequests===0;
  return{name:String(status),status:actual,targetRequests,pass:!!(pass&&forwarded),body};
 }
 try{
  const holdout=[],anchor=[];
  for(const status of [301,302,303,307,308])holdout.push(await check(status,true));
  for(const status of [200,401,429,503,300,304])anchor.push(await check(status,false));
  return{holdout,anchor,totalDestinationRequests:destinationCalls};
 }finally{for(const s of [source,target]){s.closeAllConnections();await new Promise(r=>s.close(r));}}
}
module.exports={probe};
if(require.main===module)probe(process.argv[2]||'.').then(x=>{console.log(JSON.stringify(x));if([...x.holdout,...x.anchor].some(r=>!r.pass))process.exitCode=1;}).catch(e=>{console.error(e);process.exitCode=1;});
