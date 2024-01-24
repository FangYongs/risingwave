(self.webpackChunk_N_E=self.webpackChunk_N_E||[]).push([[695],{69850:function(e,t,r){(window.__NEXT_P=window.__NEXT_P||[]).push(["/cluster",function(){return r(90915)}])},79086:function(e,t,r){"use strict";r.d(t,{OL:function(){return getClusterInfoComputeNode},X8:function(){return getClusterMetrics},xv:function(){return getClusterInfoFrontend}});var n=r(2789),i=r(34269);async function getClusterMetrics(){let e=await i.ZP.get("/metrics/cluster");return e}async function getClusterInfoFrontend(){let e=(await i.ZP.get("/clusters/1")).map(n.cX.fromJSON);return e}async function getClusterInfoComputeNode(){let e=(await i.ZP.get("/clusters/2")).map(n.cX.fromJSON);return e}},90915:function(e,t,r){"use strict";r.r(t),r.d(t,{default:function(){return Cluster}});var n=r(85893),i=r(40639),s=r(52189),o=r(66678),a=r.n(o),l=r(31351),c=r.n(l),u=r(89734),d=r.n(u),m=r(9008),h=r.n(m),x=r(67294),f=r(9253),p=r(83323),g=r(3023),j=r(75358),w=r(86108),C=r(44527),y=r(10068),b=r(79086);function WorkerNodeComponent(e){var t,r;let{workerNodeType:s,workerNode:o}=e;return(0,n.jsx)(x.Fragment,{children:(0,n.jsxs)(i.gC,{alignItems:"start",spacing:1,children:[(0,n.jsxs)(i.Ug,{children:[(0,n.jsx)(i.xu,{w:3,h:3,flex:"none",bgColor:"green.600",rounded:"full"}),(0,n.jsxs)(i.xv,{fontWeight:"medium",fontSize:"xl",textColor:"black",children:[s," #",o.id]})]}),(0,n.jsx)(i.xv,{textColor:"gray.500",m:0,children:"Running"}),(0,n.jsxs)(i.xv,{textColor:"gray.500",m:0,children:[null===(t=o.host)||void 0===t?void 0:t.host,":",null===(r=o.host)||void 0===r?void 0:r.port]})]})})}function WorkerNodeMetricsComponent(e){let{job:t,instance:r,metrics:o,isCpuMetrics:l}=e,u=(0,x.useCallback)(()=>{let e=[];if(0===o.length)return[];let t=o.at(-1).timestamp;for(let r of c()(a()(o))){for(;t-r.timestamp>0;)t-=60,e.push({timestamp:t,value:0});e.push(r),t-=60}for(;e.length<60;)e.push({timestamp:t,value:0}),t-=60;return c()(e)},[o]);return(0,n.jsx)(x.Fragment,{children:(0,n.jsxs)(i.gC,{alignItems:"start",spacing:1,children:[(0,n.jsxs)(i.xv,{textColor:"gray.500",mx:3,children:[(0,n.jsx)("b",{children:t})," ",r]}),(0,n.jsx)(f.h,{width:"100%",height:100,children:(0,n.jsxs)(p.T,{data:u(),children:[(0,n.jsx)(g.K,{dataKey:"timestamp",type:"number",domain:["dataMin","dataMax"],hide:!0}),l&&(0,n.jsx)(j.B,{type:"number",domain:[0,1],hide:!0}),(0,n.jsx)(w.u,{isAnimationActive:!1,type:"linear",dataKey:"value",strokeWidth:1,stroke:s.rS.colors.blue["500"],fill:s.rS.colors.blue["100"]})]})})]})})}function Cluster(){let[e,t]=(0,x.useState)([]),[r,s]=(0,x.useState)([]),[o,a]=(0,x.useState)(),l=(0,y.Z)();(0,x.useEffect)(()=>((async function(){try{t(await (0,b.xv)()),s(await (0,b.OL)())}catch(e){l(e)}})(),()=>{}),[l]),(0,x.useEffect)(()=>((async function(){for(;;)try{let e=await (0,b.X8)();e.cpuData=d()(e.cpuData,e=>e.metric.instance),e.memoryData=d()(e.memoryData,e=>e.metric.instance),a(e),await new Promise(e=>setTimeout(e,5e3))}catch(e){l(e,"warning");break}})(),()=>{}),[l]);let c=(0,n.jsxs)(i.xu,{p:3,children:[(0,n.jsx)(C.Z,{children:"Cluster Overview"}),(0,n.jsxs)(i.rj,{my:3,templateColumns:"repeat(3, 1fr)",gap:6,width:"full",children:[e.map(e=>(0,n.jsx)(i.P4,{w:"full",rounded:"xl",bg:"white",shadow:"md",borderWidth:1,p:6,children:(0,n.jsx)(WorkerNodeComponent,{workerNodeType:"Frontend",workerNode:e})},e.id)),r.map(e=>(0,n.jsx)(i.P4,{w:"full",rounded:"xl",bg:"white",shadow:"md",borderWidth:1,p:6,children:(0,n.jsx)(WorkerNodeComponent,{workerNodeType:"Compute",workerNode:e})},e.id))]}),(0,n.jsx)(C.Z,{children:"CPU Usage"}),(0,n.jsx)(i.MI,{my:3,columns:3,spacing:6,width:"full",children:o&&o.cpuData.map(e=>(0,n.jsx)(i.P4,{w:"full",rounded:"xl",bg:"white",shadow:"md",borderWidth:1,children:(0,n.jsx)(WorkerNodeMetricsComponent,{job:e.metric.job,instance:e.metric.instance,metrics:e.sample,isCpuMetrics:!0})},e.metric.instance))}),(0,n.jsx)(C.Z,{children:"Memory Usage"}),(0,n.jsx)(i.MI,{my:3,columns:3,spacing:6,width:"full",children:o&&o.memoryData.map(e=>(0,n.jsx)(i.P4,{w:"full",rounded:"xl",bg:"white",shadow:"md",borderWidth:1,children:(0,n.jsx)(WorkerNodeMetricsComponent,{job:e.metric.job,instance:e.metric.instance,metrics:e.sample,isCpuMetrics:!1})},e.metric.instance))})]});return(0,n.jsxs)(x.Fragment,{children:[(0,n.jsx)(h(),{children:(0,n.jsx)("title",{children:"Cluster Overview"})}),c]})}}},function(e){e.O(0,[184,340,133,284,774,888,179],function(){return e(e.s=69850)}),_N_E=e.O()}]);