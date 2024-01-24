(self.webpackChunk_N_E=self.webpackChunk_N_E||[]).push([[734],{83234:function(e,t,r){"use strict";r.d(t,{NI:function(){return m},Yp:function(){return useFormControl},lX:function(){return v}});var n=r(67294),a=r(28387),l=r(76734),i=r(32067),o=r(54520),s=r(52494),cx=(...e)=>e.filter(Boolean).join(" "),dataAttr=e=>e?"":void 0,ariaAttr=e=>!!e||void 0;function callAllHandlers(...e){return function(t){e.some(e=>(null==e||e(t),null==t?void 0:t.defaultPrevented))}}var[u,d]=(0,a.k)({name:"FormControlStylesContext",errorMessage:"useFormControlStyles returned is 'undefined'. Seems you forgot to wrap the components in \"<FormControl />\" "}),[c,p]=(0,a.k)({strict:!1,name:"FormControlContext"}),m=(0,i.Gp)(function(e,t){let r=(0,i.jC)("Form",e),a=(0,o.Lr)(e),{getRootProps:s,htmlProps:d,...p}=function(e){let{id:t,isRequired:r,isInvalid:a,isDisabled:i,isReadOnly:o,...s}=e,u=(0,n.useId)(),d=t||`field-${u}`,c=`${d}-label`,p=`${d}-feedback`,m=`${d}-helptext`,[f,h]=(0,n.useState)(!1),[v,g]=(0,n.useState)(!1),[S,E]=(0,n.useState)(!1),I=(0,n.useCallback)((e={},t=null)=>({id:m,...e,ref:(0,l.lq)(t,e=>{e&&g(!0)})}),[m]),x=(0,n.useCallback)((e={},t=null)=>({...e,ref:t,"data-focus":dataAttr(S),"data-disabled":dataAttr(i),"data-invalid":dataAttr(a),"data-readonly":dataAttr(o),id:e.id??c,htmlFor:e.htmlFor??d}),[d,i,S,a,o,c]),_=(0,n.useCallback)((e={},t=null)=>({id:p,...e,ref:(0,l.lq)(t,e=>{e&&h(!0)}),"aria-live":"polite"}),[p]),y=(0,n.useCallback)((e={},t=null)=>({...e,...s,ref:t,role:"group"}),[s]),b=(0,n.useCallback)((e={},t=null)=>({...e,ref:t,role:"presentation","aria-hidden":!0,children:e.children||"*"}),[]);return{isRequired:!!r,isInvalid:!!a,isReadOnly:!!o,isDisabled:!!i,isFocused:!!S,onFocus:()=>E(!0),onBlur:()=>E(!1),hasFeedbackText:f,setHasFeedbackText:h,hasHelpText:v,setHasHelpText:g,id:d,labelId:c,feedbackId:p,helpTextId:m,htmlProps:s,getHelpTextProps:I,getErrorMessageProps:_,getRootProps:y,getLabelProps:x,getRequiredIndicatorProps:b}}(a),m=cx("chakra-form-control",e.className);return n.createElement(c,{value:p},n.createElement(u,{value:r},n.createElement(i.m$.div,{...s({},t),className:m,__css:r.container})))});function useFormControl(e){let{isDisabled:t,isInvalid:r,isReadOnly:n,isRequired:a,...l}=function(e){let t=p(),{id:r,disabled:n,readOnly:a,required:l,isRequired:i,isInvalid:o,isReadOnly:s,isDisabled:u,onFocus:d,onBlur:c,...m}=e,f=e["aria-describedby"]?[e["aria-describedby"]]:[];return(null==t?void 0:t.hasFeedbackText)&&(null==t?void 0:t.isInvalid)&&f.push(t.feedbackId),(null==t?void 0:t.hasHelpText)&&f.push(t.helpTextId),{...m,"aria-describedby":f.join(" ")||void 0,id:r??(null==t?void 0:t.id),isDisabled:n??u??(null==t?void 0:t.isDisabled),isReadOnly:a??s??(null==t?void 0:t.isReadOnly),isRequired:l??i??(null==t?void 0:t.isRequired),isInvalid:o??(null==t?void 0:t.isInvalid),onFocus:callAllHandlers(null==t?void 0:t.onFocus,d),onBlur:callAllHandlers(null==t?void 0:t.onBlur,c)}}(e);return{...l,disabled:t,readOnly:n,required:a,"aria-invalid":ariaAttr(r),"aria-required":ariaAttr(a),"aria-readonly":ariaAttr(n)}}m.displayName="FormControl",(0,i.Gp)(function(e,t){let r=p(),a=d(),l=cx("chakra-form__helper-text",e.className);return n.createElement(i.m$.div,{...null==r?void 0:r.getHelpTextProps(e,t),__css:a.helperText,className:l})}).displayName="FormHelperText";var[f,h]=(0,a.k)({name:"FormErrorStylesContext",errorMessage:"useFormErrorStyles returned is 'undefined'. Seems you forgot to wrap the components in \"<FormError />\" "});(0,i.Gp)((e,t)=>{let r=(0,i.jC)("FormError",e),a=(0,o.Lr)(e),l=p();return(null==l?void 0:l.isInvalid)?n.createElement(f,{value:r},n.createElement(i.m$.div,{...null==l?void 0:l.getErrorMessageProps(a,t),className:cx("chakra-form__error-message",e.className),__css:{display:"flex",alignItems:"center",...r.text}})):null}).displayName="FormErrorMessage",(0,i.Gp)((e,t)=>{let r=h(),a=p();if(!(null==a?void 0:a.isInvalid))return null;let l=cx("chakra-form__error-icon",e.className);return n.createElement(s.JO,{ref:t,"aria-hidden":!0,...e,__css:r.icon,className:l},n.createElement("path",{fill:"currentColor",d:"M11.983,0a12.206,12.206,0,0,0-8.51,3.653A11.8,11.8,0,0,0,0,12.207,11.779,11.779,0,0,0,11.8,24h.214A12.111,12.111,0,0,0,24,11.791h0A11.766,11.766,0,0,0,11.983,0ZM10.5,16.542a1.476,1.476,0,0,1,1.449-1.53h.027a1.527,1.527,0,0,1,1.523,1.47,1.475,1.475,0,0,1-1.449,1.53h-.027A1.529,1.529,0,0,1,10.5,16.542ZM11,12.5v-6a1,1,0,0,1,2,0v6a1,1,0,1,1-2,0Z"}))}).displayName="FormErrorIcon";var v=(0,i.Gp)(function(e,t){let r=(0,i.mq)("FormLabel",e),a=(0,o.Lr)(e),{className:l,children:s,requiredIndicator:u=n.createElement(g,null),optionalIndicator:d=null,...c}=a,m=p(),f=(null==m?void 0:m.getLabelProps(c,t))??{ref:t,...c};return n.createElement(i.m$.label,{...f,className:cx("chakra-form__label",a.className),__css:{display:"block",textAlign:"start",...r}},s,(null==m?void 0:m.isRequired)?u:d)});v.displayName="FormLabel";var g=(0,i.Gp)(function(e,t){let r=p(),a=d();if(!(null==r?void 0:r.isRequired))return null;let l=cx("chakra-form__required-indicator",e.className);return n.createElement(i.m$.span,{...null==r?void 0:r.getRequiredIndicatorProps(e,t),__css:a.requiredIndicator,className:l})});g.displayName="RequiredIndicator"},20979:function(e,t,r){"use strict";r.d(t,{II:function(){return c}});var n=r(67294),a=r(83234),l=r(32067),i=r(54520),o=r(95336),s=r(28387),u=r(34031),d=r(46076),c=(0,l.Gp)(function(e,t){let{htmlSize:r,...s}=e,u=(0,l.jC)("Input",s),d=(0,i.Lr)(s),c=(0,a.Yp)(d),p=(0,o.cx)("chakra-input",e.className);return n.createElement(l.m$.input,{size:r,...c,__css:u.field,ref:t,className:p})});c.displayName="Input",c.id="Input";var[p,m]=(0,s.k)({name:"InputGroupStylesContext",errorMessage:"useInputGroupStyles returned is 'undefined'. Seems you forgot to wrap the components in \"<InputGroup />\" "});(0,l.Gp)(function(e,t){let r=(0,l.jC)("Input",e),{children:a,className:s,...c}=(0,i.Lr)(e),m=(0,o.cx)("chakra-input__group",s),f={},h=(0,u.W)(a),v=r.field;h.forEach(e=>{r&&(v&&"InputLeftElement"===e.type.id&&(f.paddingStart=v.height??v.h),v&&"InputRightElement"===e.type.id&&(f.paddingEnd=v.height??v.h),"InputRightAddon"===e.type.id&&(f.borderEndRadius=0),"InputLeftAddon"===e.type.id&&(f.borderStartRadius=0))});let g=h.map(t=>{var r,a;let l=(0,d.oA)({size:(null==(r=t.props)?void 0:r.size)||e.size,variant:(null==(a=t.props)?void 0:a.variant)||e.variant});return"Input"!==t.type.id?(0,n.cloneElement)(t,l):(0,n.cloneElement)(t,Object.assign(l,f,t.props))});return n.createElement(l.m$.div,{className:m,ref:t,__css:{width:"100%",display:"flex",position:"relative"},...c},n.createElement(p,{value:r},g))}).displayName="InputGroup";var f={left:{marginEnd:"-1px",borderEndRadius:0,borderEndColor:"transparent"},right:{marginStart:"-1px",borderStartRadius:0,borderStartColor:"transparent"}},h=(0,l.m$)("div",{baseStyle:{flex:"0 0 auto",width:"auto",display:"flex",alignItems:"center",whiteSpace:"nowrap"}}),v=(0,l.Gp)(function(e,t){let{placement:r="left",...a}=e,l=f[r]??{},i=m();return n.createElement(h,{ref:t,...a,__css:{...i.addon,...l}})});v.displayName="InputAddon";var g=(0,l.Gp)(function(e,t){return n.createElement(v,{ref:t,placement:"left",...e,className:(0,o.cx)("chakra-input__left-addon",e.className)})});g.displayName="InputLeftAddon",g.id="InputLeftAddon";var S=(0,l.Gp)(function(e,t){return n.createElement(v,{ref:t,placement:"right",...e,className:(0,o.cx)("chakra-input__right-addon",e.className)})});S.displayName="InputRightAddon",S.id="InputRightAddon";var E=(0,l.m$)("div",{baseStyle:{display:"flex",alignItems:"center",justifyContent:"center",position:"absolute",top:"0",zIndex:2}}),I=(0,l.Gp)(function(e,t){let{placement:r="left",...a}=e,l=m(),i=l.field,o={["left"===r?"insetStart":"insetEnd"]:"0",width:(null==i?void 0:i.height)??(null==i?void 0:i.h),height:(null==i?void 0:i.height)??(null==i?void 0:i.h),fontSize:null==i?void 0:i.fontSize,...l.element};return n.createElement(E,{ref:t,__css:o,...a})});I.id="InputElement",I.displayName="InputElement";var x=(0,l.Gp)(function(e,t){let{className:r,...a}=e,l=(0,o.cx)("chakra-input__left-element",r);return n.createElement(I,{ref:t,placement:"left",className:l,...a})});x.id="InputLeftElement",x.displayName="InputLeftElement";var _=(0,l.Gp)(function(e,t){let{className:r,...a}=e,l=(0,o.cx)("chakra-input__right-element",r);return n.createElement(I,{ref:t,placement:"right",className:l,...a})});_.id="InputRightElement",_.displayName="InputRightElement"},52837:function(e,t,r){(window.__NEXT_P=window.__NEXT_P||[]).push(["/settings",function(){return r(85046)}])},44527:function(e,t,r){"use strict";var n=r(85893),a=r(40639);r(67294),t.Z=function(e){let{children:t}=e;return(0,n.jsx)(a.xv,{mb:2,textColor:"blue.500",fontWeight:"semibold",lineHeight:"6",children:t})}},34269:function(e,t,r){"use strict";r.d(t,{Cm:function(){return i},Yg:function(){return l},e_:function(){return a}});let n="/api",a=[n,"http://localhost:32333","http://localhost:5691/api"],l=n,i="risingwave.dashboard.api.endpoint",o=new class{urlFor(e){let t=(JSON.parse(localStorage.getItem(i)||"null")||l).replace(/\/+$/,"");return"".concat(t).concat(e)}async get(e){let t=this.urlFor(e);try{let e=await fetch(t),r=await e.json();if(!e.ok)throw"".concat(e.status," ").concat(e.statusText).concat(r.error?": "+r.error:"");return r}catch(e){throw console.error(e),Error("Failed to fetch ".concat(t),{cause:e})}}};t.ZP=o},85046:function(e,t,r){"use strict";r.r(t),r.d(t,{default:function(){return Settings}});var n=r(85893),a=r(40639),l=r(83234),i=r(20979),o=r(67294);function dispatchStorageEvent(e,t){window.dispatchEvent(new StorageEvent("storage",{key:e,newValue:t}))}let setLocalStorageItem=(e,t)=>{let r=JSON.stringify(t);window.localStorage.setItem(e,r),dispatchStorageEvent(e,r)},removeLocalStorageItem=e=>{window.localStorage.removeItem(e),dispatchStorageEvent(e,null)},getLocalStorageItem=e=>window.localStorage.getItem(e),useLocalStorageSubscribe=e=>(window.addEventListener("storage",e),()=>window.removeEventListener("storage",e)),getLocalStorageServerSnapshot=()=>{throw Error("useLocalStorage is a client-only hook")};var s=r(9008),u=r.n(s),d=r(44527),c=r(34269);function Settings(){let e=function(){let[e,t]=o.useState(!1);return o.useEffect(()=>{t(!0)},[]),e}();return e&&(0,n.jsx)(ClientSettings,{})}function ClientSettings(){let[e,t]=function(e,t){let r=o.useSyncExternalStore(useLocalStorageSubscribe,()=>getLocalStorageItem(e),getLocalStorageServerSnapshot),n=o.useCallback(t=>{try{let n="function"==typeof t?t(JSON.parse(r)):t;null==n?removeLocalStorageItem(e):setLocalStorageItem(e,n)}catch(e){console.warn(e)}},[e,r]);return o.useEffect(()=>{null===getLocalStorageItem(e)&&void 0!==t&&setLocalStorageItem(e,t)},[e,t]),[r?JSON.parse(r):t,n]}(c.Cm,c.Yg);return(0,n.jsxs)(o.Fragment,{children:[(0,n.jsx)(u(),{children:(0,n.jsx)("title",{children:"Settings"})}),(0,n.jsxs)(a.xu,{p:3,children:[(0,n.jsx)(d.Z,{children:"Settings"}),(0,n.jsx)(a.gC,{spacing:4,w:"full",children:(0,n.jsxs)(l.NI,{children:[(0,n.jsx)(l.lX,{children:"RisingWave Meta Node HTTP API"}),(0,n.jsx)(i.II,{value:e,onChange:e=>t(e.target.value),list:"predefined"}),(0,n.jsx)("datalist",{id:"predefined",children:c.e_.map(e=>(0,n.jsx)("option",{value:e},e))})]})})]})]})}},9008:function(e,t,r){e.exports=r(34605)}},function(e){e.O(0,[774,888,179],function(){return e(e.s=52837)}),_N_E=e.O()}]);