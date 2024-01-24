(self.webpackChunk_N_E=self.webpackChunk_N_E||[]).push([[653],{39653:function(e,t,n){"use strict";n.d(t,{Ck:function(){return useFocusOnHide},Gp:function(){return useFocusOnShow},hS:function(){return useAnimationState},qY:function(){return useDisclosure},s9:function(){return useFocusOnPointerDown}});var r=n(72446),o=n(67294);n(20640);var u=r.jU?o.useLayoutEffect:o.useEffect;function useCallbackRef(e,t=[]){let n=(0,o.useRef)(e);return u(()=>{n.current=e}),(0,o.useCallback)((...e)=>{var t;return null==(t=n.current)?void 0:t.call(n,...e)},t)}function useEventListener(e,t,n,u){let a=useCallbackRef(t);return(0,o.useEffect)(()=>{let o=(0,r.Pu)(n)??document;if(t)return o.addEventListener(e,a,u),()=>{o.removeEventListener(e,a,u)}},[e,n,u,a,t]),()=>{let t=(0,r.Pu)(n)??document;t.removeEventListener(e,a,u)}}function useAnimationState(e){let{isOpen:t,ref:n}=e,[u,a]=(0,o.useState)(t),[c,s]=(0,o.useState)(!1);(0,o.useEffect)(()=>{c||(a(t),s(!0))},[t,c,u]),useEventListener("animationend",()=>{a(t)},()=>n.current);let l=!t&&!u;return{present:!l,onComplete(){var e;let t=(0,r.kR)(n.current),o=new t.CustomEvent("animationend",{bubbles:!0});null==(e=n.current)||e.dispatchEvent(o)}}}function useDisclosure(e={}){let{onClose:t,onOpen:n,isOpen:u,id:a}=e,c=useCallbackRef(n),s=useCallbackRef(t),[l,i]=(0,o.useState)(e.defaultIsOpen||!1),[f,d]=function(e,t){let n=void 0!==e;return[n,n&&void 0!==e?e:t]}(u,l),p=function(e,t){let n=(0,o.useId)();return(0,o.useMemo)(()=>e||[t,n].filter(Boolean).join("-"),[e,t,n])}(a,"disclosure"),m=(0,o.useCallback)(()=>{f||i(!1),null==s||s()},[f,s]),v=(0,o.useCallback)(()=>{f||i(!0),null==c||c()},[f,c]),b=(0,o.useCallback)(()=>{let e=d?m:v;e()},[d,v,m]);return{isOpen:!!d,onOpen:v,onClose:m,onToggle:b,isControlled:f,getButtonProps:(e={})=>({...e,"aria-expanded":d,"aria-controls":p,onClick:(0,r.v0)(e.onClick,b)}),getDisclosureProps:(e={})=>({...e,hidden:!d,id:p})}}var useUpdateEffect=(e,t)=>{let n=(0,o.useRef)(!1),r=(0,o.useRef)(!1);(0,o.useEffect)(()=>{let t=n.current,o=t&&r.current;if(o)return e();r.current=!0},t),(0,o.useEffect)(()=>(n.current=!0,()=>{n.current=!1}),[])};function useFocusOnHide(e,t){let{shouldFocus:n,visible:o,focusRef:u}=t,a=n&&!o;useUpdateEffect(()=>{if(!a||function(e){let t=e.current;if(!t)return!1;let n=(0,r.vY)(t);return!(!n||(0,r.r3)(t,n))&&!!(0,r.Wq)(n)}(e))return;let t=(null==u?void 0:u.current)||e.current;t&&(0,r.T_)(t,{nextTick:!0})},[a,e,u])}function useFocusOnPointerDown(e){let{ref:t,elements:n,enabled:o}=e,u=(0,r.Ao)("Safari");useEventListener((0,r.f7)("pointerdown"),(0,r.JN)(e=>{if(!u||!o)return;let a=e.target,c=(n??[t]).some(e=>{let t=(0,r.Ik)(e)?e.current:e;return(0,r.r3)(t,a)});!(0,r.H9)(a)&&c&&(e.preventDefault(),(0,r.T_)(a))},!0),()=>(0,r.lZ)(t.current),void 0)}var a={preventScroll:!0,shouldFocus:!1};function useFocusOnShow(e,t=a){let{focusRef:n,preventScroll:u,shouldFocus:c,visible:s}=t,l=(0,r.Ik)(e)?e.current:e,i=c&&s,f=(0,o.useCallback)(()=>{if(l&&i&&!(0,r.r3)(l,document.activeElement)){if(null==n?void 0:n.current)(0,r.T_)(n.current,{preventScroll:u,nextTick:!0});else{let e=(0,r.t5)(l);e.length>0&&(0,r.T_)(e[0],{preventScroll:u,nextTick:!0})}}},[i,u,l,n]);useUpdateEffect(()=>{f()},[f]),useEventListener("transitionend",f,l)}},20640:function(e,t,n){"use strict";var r=n(11742),o={"text/plain":"Text","text/html":"Url",default:"Text"};e.exports=function(e,t){var n,u,a,c,s,l,i,f,d=!1;t||(t={}),a=t.debug||!1;try{if(s=r(),l=document.createRange(),i=document.getSelection(),(f=document.createElement("span")).textContent=e,f.style.all="unset",f.style.position="fixed",f.style.top=0,f.style.clip="rect(0, 0, 0, 0)",f.style.whiteSpace="pre",f.style.webkitUserSelect="text",f.style.MozUserSelect="text",f.style.msUserSelect="text",f.style.userSelect="text",f.addEventListener("copy",function(n){if(n.stopPropagation(),t.format){if(n.preventDefault(),void 0===n.clipboardData){a&&console.warn("unable to use e.clipboardData"),a&&console.warn("trying IE specific stuff"),window.clipboardData.clearData();var r=o[t.format]||o.default;window.clipboardData.setData(r,e)}else n.clipboardData.clearData(),n.clipboardData.setData(t.format,e)}t.onCopy&&(n.preventDefault(),t.onCopy(n.clipboardData))}),document.body.appendChild(f),l.selectNodeContents(f),i.addRange(l),!document.execCommand("copy"))throw Error("copy command was unsuccessful");d=!0}catch(r){a&&console.error("unable to copy using execCommand: ",r),a&&console.warn("trying IE specific stuff");try{window.clipboardData.setData(t.format||"text",e),t.onCopy&&t.onCopy(window.clipboardData),d=!0}catch(r){a&&console.error("unable to copy using clipboardData: ",r),a&&console.error("falling back to prompt"),n="message"in t?t.message:"Copy to clipboard: #{key}, Enter",u=(/mac os x/i.test(navigator.userAgent)?"⌘":"Ctrl")+"+C",c=n.replace(/#{\s*key\s*}/g,u),window.prompt(c,e)}}finally{i&&("function"==typeof i.removeRange?i.removeRange(l):i.removeAllRanges()),f&&document.body.removeChild(f),s()}return d}},11742:function(e){e.exports=function(){var e=document.getSelection();if(!e.rangeCount)return function(){};for(var t=document.activeElement,n=[],r=0;r<e.rangeCount;r++)n.push(e.getRangeAt(r));switch(t.tagName.toUpperCase()){case"INPUT":case"TEXTAREA":t.blur();break;default:t=null}return e.removeAllRanges(),function(){"Caret"===e.type&&e.removeAllRanges(),e.rangeCount||n.forEach(function(t){e.addRange(t)}),t&&t.focus()}}}}]);