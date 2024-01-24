(self.webpackChunk_N_E=self.webpackChunk_N_E||[]).push([[763],{94469:function(e,t,r){(window.__NEXT_P=window.__NEXT_P||[]).push(["/fragment_graph",function(){return r(15193)}])},67670:function(e,t,r){"use strict";r.d(t,{A8:function(){return layoutItem},AJ:function(){return generateRelationEdges},Sx:function(){return flipLayoutRelation},jI:function(){return generateFragmentEdges}});var a=r(6162),n=r.n(a);function layoutItem(e,t,r){let a=function(e){let t=new Map;for(let r of e)t.set(r.id,r);let r=new Map,a=new Map,getNode=e=>{let n=a.get(e);if(void 0!==n)return n;let l={nextNodes:[]},o=t.get(e);if(void 0===o)throw Error("no such id ".concat(e));for(let e of o.parentIds)getNode(e).nextNodes.push(l);return a.set(e,l),r.set(l,e),l};for(let t of e)getNode(t.id);let n=new Map,l=[];for(let e of r.keys())l.push(e);for(let e of function(e){let t=[],r=[],a=new Map,visit=e=>{if(e.temp)throw Error("This is not a DAG");if(!e.perm){e.temp=!0;let r=-1;for(let t of e.node.nextNodes){a.get(t).isInput=!1,e.isOutput=!1;let n=visit(a.get(t));n>r&&(r=n)}e.temp=!1,e.perm=!0,e.g=r+1,t.unshift(e.node)}return e.g};for(let t of e){let e={node:t,temp:!1,perm:!1,isInput:!0,isOutput:!0,g:0};a.set(t,e),r.push(e)}let n=0;for(let e of r){let t=visit(e);t>n&&(n=t)}for(let e of r)e.g=n-e.g;let l=[];for(let e=0;e<n+1;++e)l.push({nodes:[],occupyRow:new Set});let o=new Map,i=new Map;for(let e of r)l[e.g].nodes.push(e.node),o.set(e.node,e.g);let putNodeInPosition=(e,t)=>{i.set(e,t),l[o.get(e)].occupyRow.add(t)},occupyLine=(e,t,r)=>{for(let a=e;a<=t;++a)l[a].occupyRow.add(r)},hasOccupied=(e,t)=>l[e].occupyRow.has(t),isStraightLineOccupied=(e,t,r)=>{if(r<0)return!1;for(let a=e;a<=t;++a)if(hasOccupied(a,r))return!0;return!1};for(let t of e)t.nextNodes.sort((e,t)=>o.get(t)-o.get(e));for(let e of l)for(let t of e.nodes){if(!i.has(t)){for(let e of t.nextNodes){if(i.has(e))continue;let r=-1;for(;isStraightLineOccupied(o.get(t),o.get(e),++r););putNodeInPosition(t,r),putNodeInPosition(e,r),occupyLine(o.get(t)+1,o.get(e)-1,r);break}if(!i.has(t)){let e=-1;for(;hasOccupied(o.get(t),++e););putNodeInPosition(t,e)}}for(let e of t.nextNodes){if(i.has(e))continue;let r=i.get(t);if(!isStraightLineOccupied(o.get(t)+1,o.get(e),r)){putNodeInPosition(e,r),occupyLine(o.get(t)+1,o.get(e)-1,r);continue}for(r=-1;isStraightLineOccupied(o.get(t)+1,o.get(e),++r););putNodeInPosition(e,r),occupyLine(o.get(t)+1,o.get(e)-1,r)}}let s=new Map;for(let t of e)s.set(t,[o.get(t),i.get(t)]);return s}(l)){let a=r.get(e[0]);if(!a)throw Error("no corresponding item of node ".concat(e[0]));let l=t.get(a);if(!l)throw Error("item id ".concat(a," is not present in idToBox"));n.set(l,e[1])}return n}(e),l=new Map,o=new Map,i=0,s=0;for(let e of a){let t=e[0],r=e[1][0],a=e[1][1],c=l.get(r)||0;t.width>c&&l.set(r,t.width);let d=o.get(a)||0;t.height>d&&o.set(a,t.height),i=n()([r,i])||0,s=n()([a,s])||0}let c=new Map,d=new Map,getCumulativeMargin=(e,t,r,a)=>{let n=r.get(e);if(n)return n;if(0===e)n=0;else{let l=a.get(e-1);if(!l)throw Error("".concat(e-1," has no result"));n=getCumulativeMargin(e-1,t,r,a)+l+t}return r.set(e,n),n};for(let e=0;e<=i;++e)getCumulativeMargin(e,t,c,l);for(let e=0;e<=s;++e)getCumulativeMargin(e,r,d,o);let u=[];for(let[e,[t,r]]of a){let a=c.get(t),n=d.get(r);if(void 0!==a&&void 0!==n)u.push({x:a,y:n,...e});else throw Error("x of layer ".concat(t,": ").concat(a,", y of row ").concat(r,": ").concat(n," "))}return u}function flipLayoutRelation(e,t,r,a){let n=function(e,t,r,a){let n=layoutItem(e,t,r);return n.map(e=>{let{x:t,y:r,...n}=e;return{x:t+a,y:r+a,...n}})}(e,r,t,a);return n.map(e=>{let{x:t,y:r,...a}=e;return{x:r,y:t,...a}})}function generateRelationEdges(e){let t=[],r=new Map;for(let t of e)r.set(t.id,t);for(let a of e)for(let e of a.parentIds){let n=r.get(e);t.push({points:[{x:a.x,y:a.y},{x:n.x,y:n.y}],source:a.id,target:e})}return t}function generateFragmentEdges(e){let t=[],r=new Map;for(let t of e)r.set(t.id,t);for(let a of e){for(let e of a.parentIds){let n=r.get(e);t.push({points:[{x:a.x+a.width/2,y:a.y+a.height/2},{x:n.x+n.width/2,y:n.y+n.height/2}],source:a.id,target:e})}for(let e of a.externalParentIds)t.push({points:[{x:a.x,y:a.y+a.height/2},{x:a.x+100,y:a.y+a.height/2}],source:a.id,target:e})}return t}},15193:function(e,t,r){"use strict";r.r(t),r.d(t,{default:function(){return Streaming}});var a=r(85893),n=r(40639),l=r(83234),o=r(20979),i=r(57026),s=r(47741),c=r(79855),d=r(97098),u=r(96486),p=r.n(u),f=r(9008),g=r.n(f),h=r(95100),m=r(67294),x=r(52189),y=r(50361),v=r.n(y);function FragmentDependencyGraph(e){let{fragmentDependency:t,svgWidth:r,selectedId:n,onSelectedIdChange:l}=e,o=(0,m.useRef)(null),[i,s]=(0,m.useState)("0px"),u=(0,m.useCallback)(()=>{let e=(0,d.zx)().nodeSize([10,34,5]),r=v()(t),{width:a,height:n}=e(r);return{width:a,height:n,dag:r}},[t]),p=u();return(0,m.useEffect)(()=>{let{width:e,height:t,dag:a}=p,i=o.current,d=c.Ys(i),u=c.ak_,f=c.jvg().curve(u).x(e=>{let{x:t}=e;return t+10}).y(e=>{let{y:t}=e;return t}),isSelected=e=>e.data.id===n,g=d.select(".edges").selectAll(".edge").data(a.links()),applyEdge=e=>e.attr("d",e=>{let{points:t}=e;return f(t)}).attr("fill","none").attr("stroke-width",e=>isSelected(e.source)||isSelected(e.target)?2:1).attr("stroke",e=>isSelected(e.source)||isSelected(e.target)?x.rS.colors.blue["500"]:x.rS.colors.gray["300"]);g.exit().remove(),g.enter().call(e=>e.append("path").attr("class","edge").call(applyEdge)),g.call(applyEdge);let h=d.select(".nodes").selectAll(".node").data(a.descendants()),applyNode=e=>e.attr("transform",e=>"translate(".concat(e.x+10,", ").concat(e.y,")")).attr("fill",e=>isSelected(e)?x.rS.colors.blue["500"]:x.rS.colors.gray["500"]);h.exit().remove(),h.enter().call(e=>e.append("circle").attr("class","node").attr("r",5).call(applyNode)),h.call(applyNode);let m=d.select(".labels").selectAll(".label").data(a.descendants()),applyLabel=e=>e.text(e=>e.data.name).attr("x",r-10).attr("font-family","inherit").attr("text-anchor","end").attr("alignment-baseline","middle").attr("y",e=>e.y).attr("fill",e=>isSelected(e)?x.rS.colors.black["500"]:x.rS.colors.gray["500"]).attr("font-weight","600");m.exit().remove(),m.enter().call(e=>e.append("text").attr("class","label").call(applyLabel)),m.call(applyLabel);let y=d.select(".overlays").selectAll(".overlay").data(a.descendants()),applyOverlay=e=>e.attr("x",3).attr("height",24).attr("width",r-6).attr("y",e=>e.y-5-12+2+3).attr("rx",5).attr("fill",x.rS.colors.gray["500"]).attr("opacity",0).style("cursor","pointer");y.exit().remove(),y.enter().call(e=>e.append("rect").attr("class","overlay").call(applyOverlay).on("mouseover",function(e,t){c.Ys(this).transition().duration(parseInt(x.rS.transition.duration.normal)).attr("opacity",".10")}).on("mouseout",function(e,t){c.Ys(this).transition().duration(parseInt(x.rS.transition.duration.normal)).attr("opacity","0")}).on("mousedown",function(e,t){c.Ys(this).transition().duration(parseInt(x.rS.transition.duration.normal)).attr("opacity",".20")}).on("mouseup",function(e,t){c.Ys(this).transition().duration(parseInt(x.rS.transition.duration.normal)).attr("opacity",".10")}).on("click",function(e,t){l&&l(t.data.id)})),y.call(applyOverlay),s("".concat(t,"px"))},[t,n,r,l,p]),(0,a.jsxs)("svg",{ref:o,width:"".concat(r,"px"),height:i,children:[(0,a.jsx)("g",{className:"edges"}),(0,a.jsx)("g",{className:"nodes"}),(0,a.jsx)("g",{className:"labels"}),(0,a.jsx)("g",{className:"overlays"})]})}var S=r(39653),w=r(50471),I=r(98032),j=r(63679),b=r(67670);let k=(0,j.ZP)(()=>r.e(171).then(r.t.bind(r,55171,23)));function FragmentGraph(e){let{planNodeDependencies:t,fragmentDependency:r,selectedFragmentId:n,backPressures:l}=e,o=(0,m.useRef)(null),{isOpen:i,onOpen:d,onClose:u}=(0,S.qY)(),[p,f]=(0,m.useState)(),g=(0,m.useCallback)(e=>{f(e),d()},[d,f]),h=(0,m.useCallback)(()=>{let e=v()(t),a=v()(r),n=new Map,l=new Set;for(let[t,r]of e){var o;let e=function(e,t){let{dx:r,dy:a}=t,n=c.G_s().nodeSize([a,r]),l=n(e);return l.each(e=>([e.x,e.y]=[e.y,e.x])),l.each(e=>e.x=-e.x),l}(r,{dx:72,dy:48}),{width:a,height:i}=function(e,t){let{margin:{top:r,bottom:a,left:n,right:l}}=t,o=1/0,i=-1/0,s=1/0,c=-1/0;return e.each(e=>i=e.x>i?e.x:i),e.each(e=>o=e.x<o?e.x:o),e.each(e=>c=e.y>c?e.y:c),e.each(e=>s=e.y<s?e.y:s),o-=n,i+=l,s-=r,c+=a,e.each(e=>e.x=e.x-o),e.each(e=>e.y=e.y-s),{width:i-o,height:c-s}}(e,{margin:{left:72,right:72,top:60,bottom:72}});n.set(t,{layoutRoot:e,width:a,height:i,actorIds:null!==(o=r.data.actorIds)&&void 0!==o?o:[]}),l.add(t)}let i=(0,b.A8)(a.map(e=>{let{width:t,height:r,id:a,...l}=e,{width:o,height:i}=n.get(a);return{width:o,height:i,id:a,...l}}),24,24),s=new Map;i.forEach(e=>{let{id:t,x:r,y:a}=e;s.set(t,{x:r,y:a})});let d=[];for(let[e,t]of n){let{x:r,y:a}=s.get(e);d.push({id:e,x:r,y:a,...t})}let u=0,p=0;d.forEach(e=>{let{x:t,y:r,width:a,height:n}=e;p=Math.max(p,r+n+50),u=Math.max(u,t+a)});let f=(0,b.jI)(i);return{layoutResult:d,svgWidth:u,svgHeight:p,edges:f,includedFragmentIds:l}},[t,r]),{svgWidth:y,svgHeight:j,edges:N,layoutResult:E,includedFragmentIds:M}=h();return(0,m.useEffect)(()=>{if(E){let e=o.current,t=c.Ys(e),r=c.h5h().x(e=>e.x).y(e=>e.y),isSelected=e=>e===n,applyFragment=e=>{e.attr("transform",e=>{let{x:t,y:r}=e;return"translate(".concat(t,", ").concat(r,")")});let t=e.select(".text-frag-id");t.empty()&&(t=e.append("text").attr("class","text-frag-id")),t.attr("fill","black").text(e=>{let{id:t}=e;return"Fragment ".concat(t)}).attr("font-family","inherit").attr("text-anchor","end").attr("dy",e=>{let{height:t}=e;return t-24+12}).attr("dx",e=>{let{width:t}=e;return t-24}).attr("fill","black").attr("font-size",12);let a=e.select(".text-actor-id");a.empty()&&(a=e.append("text").attr("class","text-actor-id")),a.attr("fill","black").text(e=>{let{actorIds:t}=e;return"Actor ".concat(t.join(", "))}).attr("font-family","inherit").attr("text-anchor","end").attr("dy",e=>{let{height:t}=e;return t-24+24}).attr("dx",e=>{let{width:t}=e;return t-24}).attr("fill","black").attr("font-size",12);let n=e.select(".bounding-box");n.empty()&&(n=e.append("rect").attr("class","bounding-box")),n.attr("width",e=>{let{width:t}=e;return t-48}).attr("height",e=>{let{height:t}=e;return t-48}).attr("x",24).attr("y",24).attr("fill","white").attr("stroke-width",e=>{let{id:t}=e;return isSelected(t)?3:1}).attr("rx",5).attr("stroke",e=>{let{id:t}=e;return isSelected(t)?x.rS.colors.blue[500]:x.rS.colors.gray[500]});let l=e.select(".edges");l.empty()&&(l=e.append("g").attr("class","edges"));let applyEdge=e=>e.attr("d",r),o=l.selectAll("path").data(e=>{let{layoutRoot:t}=e;return t.links()});o.enter().call(e=>(e.append("path").attr("fill","none").attr("stroke",x.rS.colors.gray[700]).attr("stroke-width",1.5).call(applyEdge),e)),o.call(applyEdge),o.exit().remove();let i=e.select(".nodes");i.empty()&&(i=e.append("g").attr("class","nodes"));let applyStreamNode=e=>{e.attr("transform",e=>"translate(".concat(e.x,",").concat(e.y,")"));let t=e.select("circle");t.empty()&&(t=e.append("circle")),t.attr("fill",x.rS.colors.blue[500]).attr("r",12).style("cursor","pointer").on("click",(e,t)=>g(t.data));let r=e.select("text");r.empty()&&(r=e.append("text")),r.attr("fill","black").text(e=>e.data.name).attr("font-family","inherit").attr("text-anchor","middle").attr("dy",21.6).attr("fill","black").attr("font-size",12).attr("transform","rotate(-8)");let a=e.select("title");return a.empty()&&(a=e.append("title")),a.text(e=>{var t;return null!==(t=e.data.node.identity)&&void 0!==t?t:e.data.name}),e},s=i.selectAll(".stream-node").data(e=>{let{layoutRoot:t}=e;return t.descendants()});s.exit().remove(),s.enter().call(e=>e.append("g").attr("class","stream-node").call(applyStreamNode)),s.call(applyStreamNode)},a=t.select(".fragments").selectAll(".fragment").data(E);a.enter().call(e=>e.append("g").attr("class","fragment").call(applyFragment)),a.call(applyFragment),a.exit().remove();let i=t.select(".fragment-edges").selectAll(".fragment-edge").data(N),s=c.FdL,d=c.jvg().curve(s).x(e=>{let{x:t}=e;return t}).y(e=>{let{y:t}=e;return t}),applyEdge=e=>{let t=e.select("path");t.empty()&&(t=e.append("path"));let isEdgeSelected=e=>isSelected(e.source)||isSelected(e.target);t.attr("d",e=>{let{points:t}=e;return d(t)}).attr("fill","none").attr("stroke-width",e=>{if(l){let r=l.get("".concat(e.target,"_").concat(e.source));if(r){var t;return 30*(Math.min(Math.max(r,0),100)/100)+2}}return isEdgeSelected(e)?4:2}).attr("stroke",e=>{if(l){let t=l.get("".concat(e.target,"_").concat(e.source));if(t)return function(e){let t=[x.rS.colors.green["100"],x.rS.colors.green["300"],x.rS.colors.yellow["400"],x.rS.colors.orange["500"],x.rS.colors.red["700"]].map(e=>(0,I.H)(e));e=Math.min(e=Math.max(e,0),100);let r=t.length-1,a=e/100*r,n=Math.floor(a),l=Math.ceil(a),o=(0,I.H)(t[n]).mix((0,I.H)(t[l]),(a-n)*100).toHexString();return o}(t)}return isEdgeSelected(e)?x.rS.colors.blue["500"]:x.rS.colors.gray["300"]});let r=e.select("title");return r.empty()&&(r=e.append("title")),r.text(e=>{if(l){let t=l.get("".concat(e.target,"_").concat(e.source));if(t)return"".concat(t.toFixed(2),"%")}return""}),e};i.enter().call(e=>e.append("g").attr("class","fragment-edge").call(applyEdge)),i.call(applyEdge),i.exit().remove()}},[E,N,l,n,g]),(0,a.jsxs)(m.Fragment,{children:[(0,a.jsxs)(w.u_,{isOpen:i,onClose:u,size:"5xl",children:[(0,a.jsx)(w.ZA,{}),(0,a.jsxs)(w.hz,{children:[(0,a.jsxs)(w.xB,{children:[null==p?void 0:p.operatorId," - ",null==p?void 0:p.name]}),(0,a.jsx)(w.ol,{}),(0,a.jsx)(w.fe,{children:i&&(null==p?void 0:p.node)&&(0,a.jsx)(k,{shouldCollapse:e=>{let{name:t}=e;return"input"===t||"fields"===t||"streamKey"===t},src:p.node,collapsed:3,name:null,displayDataTypes:!1})}),(0,a.jsx)(w.mz,{children:(0,a.jsx)(s.zx,{colorScheme:"blue",mr:3,onClick:u,children:"Close"})})]})]}),(0,a.jsxs)("svg",{ref:o,width:"".concat(y,"px"),height:"".concat(j,"px"),children:[(0,a.jsx)("g",{className:"fragment-edges"}),(0,a.jsx)("g",{className:"fragments"})]})]})}var N=r(44527),E=r(10068),M=r(30707),C=r(34269);async function getActorBackPressures(){let e=await C.ZP.get("/metrics/actor/back_pressures");return e}function calculatePercentile(e,t){let r=e.sort((e,t)=>e.value-t.value),a=Math.floor(r.length*t);return r[a].value}function p50(e){return calculatePercentile(e,.5)}function p90(e){return calculatePercentile(e,.9)}function p95(e){return calculatePercentile(e,.95)}function p99(e){return calculatePercentile(e,.99)}var A=r(35413);let P=["p50","p90","p95","p99"];function Streaming(){var e,t,r;let{response:u}=(0,M.Z)(A.gG),{response:f}=(0,M.Z)(A.jV),[x,y]=(0,h.v1)("id",h.U),[v,S]=(0,h.v1)("backPressure"),[w,I]=(0,m.useState)(),{response:j}=(0,M.Z)(getActorBackPressures,5e3,null!==v),b=(0,m.useCallback)(()=>{if(f&&x){let e=f.find(e=>e.tableId===x);if(e){let t=function(e){let t=[],r=new Map;for(let t in e.fragments){let a=e.fragments[t];for(let e of a.actors)r.set(e.actorId,e.fragmentId)}for(let a in e.fragments){let n=e.fragments[a],l=new Set,o=new Set;for(let e of n.actors)for(let t of e.upstreamActorId){let a=r.get(t);if(a)l.add(a);else for(let t of function(e){let t=new Set,findMergeNodesRecursive=e=>{var r;for(let a of((null===(r=e.nodeBody)||void 0===r?void 0:r.$case)==="merge"&&t.add(e.nodeBody.merge),e.input||[]))findMergeNodesRecursive(a)};return findMergeNodesRecursive(e),Array.from(t)}(e.nodes))o.add(t.upstreamFragmentId)}t.push({id:n.fragmentId.toString(),name:"Fragment ".concat(n.fragmentId),parentIds:Array.from(l).map(e=>e.toString()),externalParentIds:Array.from(o).map(e=>e.toString()),width:0,height:0,order:n.fragmentId,fragment:n})}return t}(e);return{fragments:e,fragmentDep:t,fragmentDepDag:(0,d.lu)()(t)}}}},[f,x]);(0,m.useEffect)(()=>(u&&!x&&u.length>0&&y(u[0].id),()=>{}),[x,u,y]);let k=null===(e=b())||void 0===e?void 0:e.fragmentDep,C=null===(t=b())||void 0===t?void 0:t.fragmentDepDag,_=null===(r=b())||void 0===r?void 0:r.fragments,F=(0,m.useCallback)(()=>{let e=null==_?void 0:_.fragments;if(e){let t=new Map;for(let r in e){let a=e[r],n=function(e){let t;let r=e.actors[0],hierarchyActorNode=e=>{var t,r;return{name:(null===(r=e.nodeBody)||void 0===r?void 0:null===(t=r.$case)||void 0===t?void 0:t.toString())||"unknown",children:(e.input||[]).map(hierarchyActorNode),operatorId:e.operatorId,node:e}};if(r.dispatcher.length>0){let e=p().camelCase(r.dispatcher[0].type.replace(/^DISPATCHER_TYPE_/,""));t=r.dispatcher.length>1?r.dispatcher.every(e=>e.type===r.dispatcher[0].type)?"".concat(e,"Dispatchers"):"multipleDispatchers":"".concat(e,"Dispatcher")}else t="noDispatcher";let a=e.actors.reduce((e,t)=>(e[t.actorId]=t.dispatcher,e),{});return c.bT9({name:t,actorIds:e.actors.map(e=>e.actorId.toString()),children:r.nodes?[hierarchyActorNode(r.nodes)]:[],operatorId:"dispatcher",node:a})}(a);t.set(r,n)}return t}},[null==_?void 0:_.fragments]),D=F(),[L,R]=(0,m.useState)(""),[O,z]=(0,m.useState)(""),G=(0,E.Z)(),handleSearchFragment=()=>{let e=parseInt(O);if(f){for(let t of f)for(let r in t.fragments)if(t.fragments[r].fragmentId==e){y(t.tableId),I(e);return}}G(Error("Fragment ".concat(e," not found")))},handleSearchActor=()=>{let e=parseInt(L);if(f)for(let t of f)for(let r in t.fragments){let a=t.fragments[r];for(let r of a.actors)if(r.actorId==e){y(t.tableId),I(a.fragmentId);return}}G(Error("Actor ".concat(e," not found")))},B=(0,m.useMemo)(()=>{if(j&&v){let e=new Map;for(let t of j.outputBufferBlockingDuration){let r;switch(console.log(v),v){case"p50":r=p50;break;case"p90":r=p90;break;case"p95":r=p95;break;case"p99":r=p99;break;default:return}let a=100*r(t.sample);e.set("".concat(t.metric.fragment_id,"_").concat(t.metric.downstream_fragment_id),a)}return e}},[j,v]),Y=(0,a.jsxs)(n.kC,{p:3,height:"calc(100vh - 20px)",flexDirection:"column",children:[(0,a.jsx)(N.Z,{children:"Fragment Graph"}),(0,a.jsxs)(n.kC,{flexDirection:"row",height:"full",width:"full",children:[(0,a.jsxs)(n.gC,{mr:3,spacing:3,alignItems:"flex-start",width:200,height:"full",children:[(0,a.jsxs)(l.NI,{children:[(0,a.jsx)(l.lX,{children:"Relations"}),(0,a.jsx)(o.II,{list:"relationList",spellCheck:!1,onChange:e=>{var t;let r=null==u?void 0:null===(t=u.find(t=>t.name==e.target.value))||void 0===t?void 0:t.id;r&&y(r)},placeholder:"Search...",mb:2}),(0,a.jsx)("datalist",{id:"relationList",children:u&&u.map(e=>(0,a.jsxs)("option",{value:e.name,children:["(",e.id,") ",e.name]},e.id))}),(0,a.jsx)(i.Ph,{value:null!=x?x:void 0,onChange:e=>y(parseInt(e.target.value)),children:u&&u.map(e=>(0,a.jsxs)("option",{value:e.id,children:["(",e.id,") ",e.name]},e.name))})]}),(0,a.jsxs)(l.NI,{children:[(0,a.jsx)(l.lX,{children:"Goto"}),(0,a.jsxs)(n.gC,{spacing:2,children:[(0,a.jsxs)(n.Ug,{children:[(0,a.jsx)(o.II,{placeholder:"Fragment Id",value:O,onChange:e=>z(e.target.value)}),(0,a.jsx)(s.zx,{onClick:e=>handleSearchFragment(),children:"Go"})]}),(0,a.jsxs)(n.Ug,{children:[(0,a.jsx)(o.II,{placeholder:"Actor Id",value:L,onChange:e=>R(e.target.value)}),(0,a.jsx)(s.zx,{onClick:e=>handleSearchActor(),children:"Go"})]})]})]}),(0,a.jsxs)(l.NI,{children:[(0,a.jsx)(l.lX,{children:"Back Pressure"}),(0,a.jsxs)(i.Ph,{value:null!=v?v:void 0,onChange:e=>S("disabled"===e.target.value?null:e.target.value),children:[(0,a.jsx)("option",{value:"disabled",children:"Disabled"}),P.map(e=>(0,a.jsx)("option",{value:e,children:e},e))]})]}),(0,a.jsxs)(n.kC,{height:"full",width:"full",flexDirection:"column",children:[(0,a.jsx)(n.xv,{fontWeight:"semibold",children:"Fragments"}),C&&(0,a.jsx)(n.xu,{flex:"1",overflowY:"scroll",children:(0,a.jsx)(FragmentDependencyGraph,{svgWidth:200,fragmentDependency:C,onSelectedIdChange:e=>I(parseInt(e)),selectedId:null==w?void 0:w.toString()})})]})]}),(0,a.jsxs)(n.xu,{flex:1,height:"full",ml:3,overflowX:"scroll",overflowY:"scroll",children:[(0,a.jsx)(n.xv,{fontWeight:"semibold",children:"Fragment Graph"}),D&&k&&(0,a.jsx)(FragmentGraph,{selectedFragmentId:null==w?void 0:w.toString(),fragmentDependency:k,planNodeDependencies:D,backPressures:B})]})]})]});return(0,a.jsxs)(m.Fragment,{children:[(0,a.jsx)(g(),{children:(0,a.jsx)("title",{children:"Streaming Fragments"})}),Y]})}}},function(e){e.O(0,[662,679,184,145,591,855,653,340,575,284,30,774,888,179],function(){return e(e.s=94469)}),_N_E=e.O()}]);