/// The complete web UI for the NodeDB Debug Inspector.
///
/// A single HTML document with inline CSS and JavaScript.
/// Served at GET / by the InspectorServer.
const String inspectorHtml = r'''
<!DOCTYPE html>
<html lang="en">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>NodeDB Inspector</title>
<style>
:root{--bg:#1a1b26;--surface:#24283b;--border:#414868;--text:#a9b1d6;--text-bright:#c0caf5;--accent:#7aa2f7;--accent2:#bb9af7;--green:#9ece6a;--red:#f7768e;--yellow:#e0af68;--sidebar-w:200px;--font:ui-monospace,SFMono-Regular,Menlo,monospace}
*{margin:0;padding:0;box-sizing:border-box}
body{font-family:var(--font);background:var(--bg);color:var(--text);font-size:13px;height:100vh;overflow:hidden}
#app{display:grid;grid-template-columns:var(--sidebar-w) 1fr;grid-template-rows:44px 1fr;height:100vh}
header{grid-column:1/-1;background:var(--surface);border-bottom:1px solid var(--border);display:flex;align-items:center;padding:0 16px;gap:12px}
header h1{font-size:14px;color:var(--text-bright);font-weight:600}
header .dot{width:8px;height:8px;border-radius:50%;background:var(--red)}
header .dot.connected{background:var(--green)}
header .version{margin-left:auto;color:var(--border);font-size:11px}
nav{background:var(--surface);border-right:1px solid var(--border);overflow-y:auto;padding:8px 0}
nav button{display:block;width:100%;text-align:left;padding:8px 16px;background:none;border:none;color:var(--text);cursor:pointer;font-family:var(--font);font-size:12px}
nav button:hover{background:rgba(122,162,247,0.1)}
nav button.active{color:var(--accent);background:rgba(122,162,247,0.15);border-right:2px solid var(--accent)}
main{overflow-y:auto;padding:20px}
.cards{display:grid;grid-template-columns:repeat(auto-fill,minmax(180px,1fr));gap:12px;margin-bottom:20px}
.card{background:var(--surface);border:1px solid var(--border);border-radius:6px;padding:14px}
.card .label{font-size:11px;color:var(--border);text-transform:uppercase;margin-bottom:4px}
.card .value{font-size:22px;color:var(--text-bright);font-weight:600}
table{width:100%;border-collapse:collapse;margin-top:12px}
th,td{text-align:left;padding:6px 10px;border-bottom:1px solid var(--border);font-size:12px}
th{color:var(--accent);font-weight:600;font-size:11px;text-transform:uppercase;position:sticky;top:0;background:var(--bg)}
td{color:var(--text)}
tr:hover td{background:rgba(122,162,247,0.05)}
.toolbar{display:flex;gap:8px;align-items:center;margin-bottom:12px;flex-wrap:wrap}
select,input[type=text],input[type=number]{background:var(--surface);border:1px solid var(--border);color:var(--text-bright);padding:5px 8px;border-radius:4px;font-family:var(--font);font-size:12px}
button.btn{background:var(--accent);color:var(--bg);border:none;padding:5px 12px;border-radius:4px;cursor:pointer;font-family:var(--font);font-size:12px}
button.btn:hover{opacity:0.85}
button.btn.danger{background:var(--red)}
.section-title{font-size:13px;color:var(--text-bright);font-weight:600;margin:16px 0 8px}
.json-view{background:var(--surface);border:1px solid var(--border);border-radius:4px;padding:10px;font-size:12px;overflow-x:auto;white-space:pre-wrap;max-height:400px;overflow-y:auto;color:var(--text)}
.bar-row{display:flex;align-items:center;gap:8px;margin:4px 0}
.bar-row .bar-label{width:120px;font-size:11px;color:var(--text);text-align:right;overflow:hidden;text-overflow:ellipsis;white-space:nowrap}
.bar-row .bar{height:18px;background:var(--accent);border-radius:2px;min-width:2px;transition:width .3s}
.bar-row .bar-val{font-size:11px;color:var(--border);min-width:30px}
.empty{color:var(--border);font-style:italic;padding:20px;text-align:center}
.panel{display:none}.panel.active{display:block}
#auth-modal{position:fixed;inset:0;background:rgba(0,0,0,0.7);display:flex;align-items:center;justify-content:center;z-index:100}
#auth-modal .box{background:var(--surface);border:1px solid var(--border);border-radius:8px;padding:24px;text-align:center}
#auth-modal input{margin:12px 0;width:200px;text-align:center}
.heatmap-cell{display:inline-block;padding:4px 8px;margin:2px;border-radius:3px;font-size:11px}
</style>
</head>
<body>
<div id="app">
<header>
<h1>NodeDB Inspector</h1>
<span class="dot" id="status-dot"></span>
<span class="version" id="version-label"></span>
</header>
<nav id="sidebar"></nav>
<main id="content"></main>
</div>
<div id="auth-modal" style="display:none">
<div class="box">
<div style="margin-bottom:8px;color:var(--text-bright)">Inspector Passcode</div>
<input type="text" id="auth-input" placeholder="Enter passcode">
<br><button class="btn" onclick="submitAuth()">Connect</button>
</div>
</div>
<script>
const panels=['dashboard','nosql','graph','vector','federation','dac','provenance','keyResolver','schema','triggers','singletons','preferences','accessHistory'];
const panelLabels={dashboard:'Dashboard',nosql:'NoSQL',graph:'Graph',vector:'Vector',federation:'Federation',dac:'DAC',provenance:'Provenance',keyResolver:'Keys',schema:'Schema',triggers:'Triggers',singletons:'Singletons',preferences:'Preferences',accessHistory:'Access History'};
let ws,snap={},currentPanel='dashboard',enabledPanels=[];

// Auth
const params=new URLSearchParams(location.search);
const passcode=params.get('passcode');

function init(){
  buildSidebar();
  buildPanels();
  connect();
}

function connect(){
  const proto=location.protocol==='https:'?'wss:':'ws:';
  ws=new WebSocket(proto+'//'+location.host+'/ws');
  ws.onopen=()=>{
    if(passcode){ws.send(JSON.stringify({auth:passcode}))}
    else if(!passcode){document.getElementById('status-dot').classList.add('connected')}
  };
  ws.onmessage=e=>{
    const msg=JSON.parse(e.data);
    if(msg.ok===true){document.getElementById('auth-modal').style.display='none';document.getElementById('status-dot').classList.add('connected');return}
    if(msg.ok===false){alert('Bad passcode');return}
    if(msg.cmd==='snapshot'){snap=msg.data;renderCurrent()}
    if(msg.cmd==='enabledPanels'){enabledPanels=msg.data}
    if(msg.cmd==='panel'){handlePanelResponse(msg.data)}
  };
  ws.onclose=()=>{document.getElementById('status-dot').classList.remove('connected');setTimeout(connect,3000)};
}

function send(cmd,extra){ws&&ws.readyState===1&&ws.send(JSON.stringify({cmd,...extra||{}}))}
function sendPanel(panel,action,extra){send('panel',{panel,action,...extra||{}})}
function submitAuth(){const v=document.getElementById('auth-input').value;ws&&ws.send(JSON.stringify({auth:v}))}

function buildSidebar(){
  const nav=document.getElementById('sidebar');
  panels.forEach(p=>{
    const b=document.createElement('button');
    b.textContent=panelLabels[p]||p;
    b.dataset.panel=p;
    b.onclick=()=>showPanel(p);
    if(p==='dashboard')b.classList.add('active');
    nav.appendChild(b);
  });
}

function buildPanels(){
  const main=document.getElementById('content');
  panels.forEach(p=>{
    const d=document.createElement('div');
    d.className='panel'+(p==='dashboard'?' active':'');
    d.id='panel-'+p;
    main.appendChild(d);
  });
}

function showPanel(name){
  currentPanel=name;
  document.querySelectorAll('nav button').forEach(b=>b.classList.toggle('active',b.dataset.panel===name));
  document.querySelectorAll('.panel').forEach(d=>d.classList.toggle('active',d.id==='panel-'+name));
  renderCurrent();
}

function renderCurrent(){
  if(!snap)return;
  document.getElementById('version-label').textContent='v'+((snap.version||0));
  const r=renderers[currentPanel];
  if(r)r(document.getElementById('panel-'+currentPanel),snap);
}

let _panelCb=null;
function handlePanelResponse(data){if(_panelCb){const cb=_panelCb;_panelCb=null;cb(data)}}
function fetchPanel(panel,action,extra){return new Promise(r=>{_panelCb=r;sendPanel(panel,action,extra)})}

// Renderers
const renderers={};

renderers.dashboard=function(el,s){
  const n=s.nosql||{};const g=s.graph||{};const f=s.federation||{};const p=s.provenance||{};
  const k=s.keyResolver||{};const t=s.triggers||{};const si=s.singletons||{};const ah=s.accessHistory||{};
  el.innerHTML=`<div class="cards">
${card('Documents',n.totalDocuments||0)}
${card('Collections',(n.collections?Object.keys(n.collections).length:0))}
${card('Graph Nodes',g.nodeCount||0)}
${card('Peers',f.peerCount||0)}
${card('Groups',f.groupCount||0)}
${card('DAC Rules',(s.dac||{}).ruleCount||0)}
${card('Provenance',p.envelopeCount||0)}
${card('Keys',k.keyCount||0)}
${card('Triggers',t.triggerCount||0)}
${card('Singletons',(si.names||[]).length)}
${card('Access Events',ah.totalEntries||0)}
</div>
${s.nosql?'<div class="section-title">Collections</div>'+mapTable(n.collections||{}):''}
${ah.heatmap&&Object.keys(ah.heatmap).length?'<div class="section-title">Access Heatmap</div>'+barChart(ah.heatmap):''}`;
};

renderers.nosql=function(el,s){
  const cols=Object.keys((s.nosql||{}).collections||{});
  el.innerHTML=`<div class="toolbar">
<select id="nosql-col">${cols.map(c=>'<option>'+esc(c)+'</option>').join('')}</select>
<button class="btn" onclick="loadNosqlDocs()">Load</button>
<input type="number" id="nosql-limit" value="20" style="width:60px" placeholder="limit">
<input type="number" id="nosql-offset" value="0" style="width:60px" placeholder="offset">
</div><div id="nosql-docs"></div>`;
};

async function loadNosqlDocs(){
  const col=document.getElementById('nosql-col').value;
  const limit=parseInt(document.getElementById('nosql-limit').value)||20;
  const offset=parseInt(document.getElementById('nosql-offset').value)||0;
  const data=await fetchPanel('nosql','documentPreview',{collection:col,limit,offset});
  const el=document.getElementById('nosql-docs');
  if(!data||!data.length){el.innerHTML='<div class="empty">No documents</div>';return}
  el.innerHTML=`<table><tr><th>ID</th><th>Data</th><th>Created</th><th>Updated</th></tr>
${data.map(d=>`<tr><td>${d.id}</td><td class="json-view" style="max-height:60px;padding:4px">${esc(JSON.stringify(d.data).slice(0,200))}</td><td>${shortDate(d.createdAt)}</td><td>${shortDate(d.updatedAt)}</td></tr>`).join('')}</table>`;
}

renderers.graph=function(el,s){
  const g=s.graph;
  if(!g){el.innerHTML='<div class="empty">Graph engine not enabled</div>';return}
  el.innerHTML=`<div class="cards">${card('Nodes',g.nodeCount||0)}</div>
<div class="toolbar"><button class="btn" onclick="loadGraphNodes()">Load Nodes</button></div>
<div id="graph-nodes"></div>
<div class="section-title">Traversal</div>
<div class="toolbar">
<input type="number" id="graph-start" placeholder="Start ID" style="width:80px">
<select id="graph-algo"><option>bfs</option><option>dfs</option></select>
<input type="number" id="graph-depth" value="10" style="width:60px">
<button class="btn" onclick="runTraversal()">Run</button>
</div><div id="graph-traversal"></div>`;
};

async function loadGraphNodes(){
  const data=await fetchPanel('graph','nodePreview',{limit:50});
  document.getElementById('graph-nodes').innerHTML=data&&data.length?
    `<table><tr><th>ID</th><th>Label</th><th>Data</th></tr>${data.map(n=>`<tr><td>${n.id}</td><td>${esc(n.label)}</td><td>${esc(JSON.stringify(n.data).slice(0,150))}</td></tr>`).join('')}</table>`
    :'<div class="empty">No nodes</div>';
}

async function runTraversal(){
  const startId=parseInt(document.getElementById('graph-start').value);
  const algo=document.getElementById('graph-algo').value;
  const depth=parseInt(document.getElementById('graph-depth').value)||10;
  if(!startId){return}
  const data=await fetchPanel('graph','traversal',{startId,algorithm:algo,maxDepth:depth});
  document.getElementById('graph-traversal').innerHTML=`<div class="json-view">${esc(JSON.stringify(data,null,2))}</div>`;
}

renderers.vector=function(el,s){
  const v=s.vector;
  if(!v){el.innerHTML='<div class="empty">Vector engine not enabled</div>';return}
  el.innerHTML=`<div class="cards">${card('Records',v.count||0)}${card('Dimension',v.dimension||0)}${card('Metric',v.metric||'?')}</div>
<div class="section-title">Search</div>
<div class="toolbar">
<input type="text" id="vec-query" placeholder="0.1, 0.2, 0.3, ..." style="width:300px">
<input type="number" id="vec-k" value="5" style="width:50px">
<button class="btn" onclick="runVecSearch()">Search</button>
</div><div id="vec-results"></div>`;
};

async function runVecSearch(){
  const q=document.getElementById('vec-query').value.split(',').map(Number);
  const k=parseInt(document.getElementById('vec-k').value)||5;
  const data=await fetchPanel('vector','search',{query:q,k});
  document.getElementById('vec-results').innerHTML=data&&data.length?
    `<table><tr><th>ID</th><th>Distance</th><th>Metadata</th></tr>${data.map(r=>`<tr><td>${r.id}</td><td>${r.distance.toFixed(4)}</td><td>${esc(JSON.stringify(r.metadata).slice(0,150))}</td></tr>`).join('')}</table>`
    :'<div class="empty">No results</div>';
}

renderers.federation=function(el,s){
  const f=s.federation;
  if(!f){el.innerHTML='<div class="empty">Federation not available</div>';return}
  el.innerHTML=`<div class="cards">${card('Peers',f.peerCount||0)}${card('Groups',f.groupCount||0)}</div>
<button class="btn" onclick="loadFedTopology()">Load Topology</button>
<div id="fed-data"></div>`;
};

async function loadFedTopology(){
  const data=await fetchPanel('federation','topology');
  document.getElementById('fed-data').innerHTML=`<div class="json-view">${esc(JSON.stringify(data,null,2))}</div>`;
}

renderers.dac=function(el,s){
  const d=s.dac;
  if(!d){el.innerHTML='<div class="empty">DAC engine not enabled</div>';return}
  el.innerHTML=`<div class="cards">${card('Rules',d.ruleCount||0)}</div>
<button class="btn" onclick="loadDacRules()">Load Rules</button><div id="dac-rules"></div>`;
};

async function loadDacRules(){
  const data=await fetchPanel('dac','ruleList');
  document.getElementById('dac-rules').innerHTML=data&&data.length?
    `<table><tr><th>ID</th><th>Collection</th><th>Subject</th><th>Permission</th><th>Field</th></tr>
${data.map(r=>`<tr><td>${r.id}</td><td>${esc(r.collection)}</td><td>${r.subjectType}:${esc(r.subjectId)}</td><td>${r.permission}</td><td>${r.field||'*'}</td></tr>`).join('')}</table>`
    :'<div class="empty">No rules</div>';
}

renderers.provenance=function(el,s){
  const p=s.provenance;
  if(!p){el.innerHTML='<div class="empty">Provenance engine not enabled</div>';return}
  el.innerHTML=`<div class="cards">${card('Envelopes',p.envelopeCount||0)}</div>
${p.bySourceType?'<div class="section-title">By Source Type</div>'+barChart(p.bySourceType):''}
${p.byVerificationStatus?'<div class="section-title">By Verification Status</div>'+barChart(p.byVerificationStatus):''}
<button class="btn" onclick="loadRecentEnvelopes()">Recent Envelopes</button><div id="prov-recent"></div>`;
};

async function loadRecentEnvelopes(){
  const data=await fetchPanel('provenance','recentEnvelopes',{limit:20});
  document.getElementById('prov-recent').innerHTML=data&&data.length?
    `<table><tr><th>ID</th><th>Collection</th><th>Record</th><th>Source</th><th>Status</th><th>Confidence</th></tr>
${data.map(e=>`<tr><td>${e.id}</td><td>${esc(e.collection)}</td><td>${e.recordId}</td><td>${e.sourceType}</td><td>${e.verificationStatus}</td><td>${(e.confidenceFactor||0).toFixed(2)}</td></tr>`).join('')}</table>`
    :'<div class="empty">No envelopes</div>';
}

renderers.keyResolver=function(el,s){
  const k=s.keyResolver;
  if(!k){el.innerHTML='<div class="empty">KeyResolver engine not enabled</div>';return}
  el.innerHTML=`<div class="cards">${card('Keys',k.keyCount||0)}${card('Trust-All',k.trustAllActive?'ON':'OFF')}</div>
${k.byTrustLevel?'<div class="section-title">By Trust Level</div>'+barChart(k.byTrustLevel):''}
<button class="btn" onclick="loadKeys()">Load Keys</button><div id="kr-keys"></div>`;
};

async function loadKeys(){
  const data=await fetchPanel('keyResolver','keyList');
  document.getElementById('kr-keys').innerHTML=data&&data.length?
    `<table><tr><th>ID</th><th>PKI ID</th><th>User</th><th>Trust</th><th>Created</th></tr>
${data.map(k=>`<tr><td>${k.id}</td><td>${esc(k.pkiId)}</td><td>${esc(k.userId)}</td><td>${k.trustLevel}</td><td>${shortDate(k.createdAtUtc)}</td></tr>`).join('')}</table>`
    :'<div class="empty">No keys</div>';
}

renderers.schema=function(el,s){
  const n=s.nosql||{};
  el.innerHTML=`<div class="cards">${card('Fingerprint',(n.schemaFingerprint||'').slice(0,12)+'...')}</div>
<button class="btn" onclick="loadSchemaOverview()">Load Schema</button><div id="schema-data"></div>`;
};

async function loadSchemaOverview(){
  const data=await fetchPanel('schema','overview');
  document.getElementById('schema-data').innerHTML=`<div class="json-view">${esc(JSON.stringify(data,null,2))}</div>`;
}

renderers.triggers=function(el,s){
  const t=s.triggers||{};
  el.innerHTML=`<div class="cards">${card('Total',t.triggerCount||0)}${card('Enabled',t.enabledCount||0)}${card('Disabled',t.disabledCount||0)}</div>
<button class="btn" onclick="loadTriggers()">Load Triggers</button><div id="trigger-list"></div>`;
};

async function loadTriggers(){
  const data=await fetchPanel('triggers','listTriggers');
  document.getElementById('trigger-list').innerHTML=data&&data.length?
    `<table><tr><th>ID</th><th>Collection</th><th>Event</th><th>Timing</th><th>Enabled</th><th>Name</th></tr>
${data.map(t=>`<tr><td>${t.trigger_id||t.id||''}</td><td>${esc(t.collection||'')}</td><td>${t.event||''}</td><td>${t.timing||''}</td><td>${t.enabled!==false?'yes':'no'}</td><td>${esc(t.name||'')}</td></tr>`).join('')}</table>`
    :'<div class="empty">No triggers</div>';
}

renderers.singletons=function(el,s){
  const si=s.singletons||{};
  el.innerHTML=`<div class="cards">${card('Count',(si.names||[]).length)}</div>
<button class="btn" onclick="loadSingletons()">Load Singletons</button><div id="singleton-list"></div>`;
};

async function loadSingletons(){
  const data=await fetchPanel('singletons','singletonPreview');
  document.getElementById('singleton-list').innerHTML=data&&data.length?
    data.map(s=>`<div class="section-title">${esc(s.collection||'')}</div><div class="json-view">${esc(JSON.stringify(s.data||{},null,2))}</div>`).join('')
    :'<div class="empty">No singletons</div>';
}

renderers.preferences=function(el,s){
  el.innerHTML=`<div class="toolbar">
<input type="text" id="pref-store" placeholder="Store name" style="width:200px">
<button class="btn" onclick="loadPrefKeys()">Load Keys</button>
</div><div id="pref-data"></div>`;
};

async function loadPrefKeys(){
  const store=document.getElementById('pref-store').value;
  if(!store)return;
  const data=await fetchPanel('preferences','allValues',{store});
  document.getElementById('pref-data').innerHTML=data?`<div class="json-view">${esc(JSON.stringify(data,null,2))}</div>`:'<div class="empty">No data</div>';
}

renderers.accessHistory=function(el,s){
  const ah=s.accessHistory||{};
  el.innerHTML=`<div class="cards">${card('Total Events',ah.totalEntries||0)}</div>
${ah.heatmap&&Object.keys(ah.heatmap).length?'<div class="section-title">Access Heatmap</div>'+barChart(ah.heatmap):''}
<div class="section-title">Query</div>
<div class="toolbar">
<input type="text" id="ah-col" placeholder="Collection" style="width:150px">
<input type="text" id="ah-event" placeholder="Event type" style="width:120px">
<button class="btn" onclick="loadAccessHistory()">Query</button>
</div><div id="ah-results"></div>`;
};

async function loadAccessHistory(){
  const col=document.getElementById('ah-col').value||undefined;
  const evt=document.getElementById('ah-event').value||undefined;
  const data=await fetchPanel('accessHistory','query',{collection:col,eventType:evt});
  document.getElementById('ah-results').innerHTML=data&&data.length?
    `<table><tr><th>Collection</th><th>Record</th><th>Event</th><th>Time</th><th>Scope</th></tr>
${data.map(e=>`<tr><td>${esc(e.collection||'')}</td><td>${e.record_id||''}</td><td>${e.event_type||''}</td><td>${shortDate(e.accessed_at_utc||'')}</td><td>${e.query_scope||''}</td></tr>`).join('')}</table>`
    :'<div class="empty">No entries</div>';
}

// Helpers
function card(label,value){return `<div class="card"><div class="label">${esc(label)}</div><div class="value">${esc(String(value))}</div></div>`}

function mapTable(obj){
  const keys=Object.keys(obj);
  if(!keys.length)return '<div class="empty">Empty</div>';
  return `<table><tr><th>Name</th><th>Count</th></tr>${keys.map(k=>`<tr><td>${esc(k)}</td><td>${obj[k]}</td></tr>`).join('')}</table>`;
}

function barChart(obj){
  const entries=Object.entries(obj).sort((a,b)=>b[1]-a[1]);
  const max=Math.max(...entries.map(e=>e[1]),1);
  return entries.map(([k,v])=>`<div class="bar-row"><span class="bar-label">${esc(k)}</span><div class="bar" style="width:${Math.round(v/max*200)}px"></div><span class="bar-val">${v}</span></div>`).join('');
}

function esc(s){return String(s).replace(/&/g,'&amp;').replace(/</g,'&lt;').replace(/>/g,'&gt;').replace(/"/g,'&quot;')}

function shortDate(s){if(!s)return '';try{const d=new Date(s);return d.toLocaleString()}catch(e){return s}}

// Check if passcode modal is needed
window.onload=function(){
  init();
  // Show auth modal if server requires passcode and none provided via URL
  // The server will respond with ok:false if auth fails
  if(passcode){document.getElementById('auth-modal').style.display='none'}
};
</script>
</body>
</html>
''';
