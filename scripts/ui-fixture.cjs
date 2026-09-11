// Browser-only Tauri responses shared by the interaction and appearance checks.
module.exports = `
import React from '/node_modules/.vite/deps/react.js';
import ReactDOM from '/node_modules/.vite/deps/react-dom_client.js';
import {mockIPC,mockWindows} from '/node_modules/@tauri-apps/api/mocks.js';
import {emit} from '/node_modules/@tauri-apps/api/event.js';
import App from '/src/App.jsx';
import '/src/styles/variables.css';import '/src/styles/global.css';import '/src/styles/animations.css';
const scenario=new URLSearchParams(location.search).get('state')||'ready';
const state=window.testState={scenario,transfers:{},calls:[],promises:{},proton:!['first','missing'].includes(scenario),installed:!['first','disk-full'].includes(scenario),language:scenario==='en'?'en-us':'ru-ru',visible:true,minimized:false,mediaCalls:[],auto:false,saveError:false,planError:false,logError:false,offline:scenario==='offline'};
window.emit=emit;
if(scenario!=='real-video'){
let mediaPaused=true,attempt=0;
Object.defineProperty(HTMLMediaElement.prototype,'paused',{get:()=>mediaPaused,configurable:true});
HTMLMediaElement.prototype.play=function(){state.mediaCalls.push('play');attempt++;if(state.abortNext){state.abortNext=false;return Promise.reject(new DOMException('Interrupted','AbortError'));}mediaPaused=false;this.dispatchEvent(new Event('playing'));return Promise.resolve();};
HTMLMediaElement.prototype.pause=function(){state.mediaCalls.push('pause');mediaPaused=true;};
HTMLMediaElement.prototype.load=function(){state.mediaCalls.push('load');};
}
let settings={language:state.language,game_dir:'/home/player/Games/ArknightsEndfield',download_dir:'/home/player/Games/ArknightsEndfield/_download',proton_dir:state.proton?'/home/player/.local/share/llauncher/proton/dwproton-10.0-26':'',proton_prefix_dir:'',installed_version:state.installed?'1.0':'',mods_enabled:true,use_native_vulkan:true,use_wayland:true,use_sdl_input:true,use_mangohud:false,use_gamemode:false,use_gamescope:false,use_discord_rpc:false,custom_env_vars:'',custom_launch_args:'',download_speed_limit:0,download_max_concurrent:4,on_launch_action:'nothing',total_playtime_secs:state.installed?86400:0,last_played:state.installed?1788972400:0};
state.settings=settings;
if(scenario==='paused')state.transfers.game={id:'saved',slot:'game',kind:'update',status:'paused',game_dir:settings.game_dir,progress:{stage:'downloading',bytes_downloaded:20000000000,bytes_total:40000000000}};
state.progress=async(slot,progress)=>{state.transfers[slot]={...state.transfers[slot],progress};await emit('task://changed',state.transfers[slot]);};
state.complete=async(slot)=>{const task=state.transfers[slot];if(slot==='proton'){state.proton=true;settings.proton_dir='/home/player/.local/share/llauncher/proton/dwproton-10.0-26';}else{state.installed=true;settings.installed_version='1.1';}state.transfers[slot]={...task,status:'completed',result:slot==='game'?{checked:640,repaired:2}:null};await emit('task://changed',state.transfers[slot]);state.promises[slot]?.resolve();};
const bg='data:image/svg+xml,'+encodeURIComponent('<svg xmlns="http://www.w3.org/2000/svg" width="1280" height="720"><defs><linearGradient id="g" x2="1" y2="1"><stop stop-color="#6f8373"/><stop offset="1" stop-color="#253b2d"/></linearGradient></defs><path fill="url(#g)" d="M0 0h1280v720H0z"/><path fill="#91a28b" opacity=".4" d="M580 720 850 100 910 420 1030 240 1280 720Z"/></svg>');
mockWindows('main');mockIPC((cmd,args)=>{state.calls.push({cmd,args});switch(cmd){
case 'get_settings':return {...settings};case 'save_settings':if(state.saveError)return Promise.reject('Write denied');settings={...args.settings};state.settings=settings;return;
case 'get_transfers':if(scenario==='task-error'&&!state.tasksRecovered)return Promise.reject('IPC unavailable');return structuredClone(state.transfers);
case 'get_launcher_content':if(scenario==='no-news')return Promise.reject('News unavailable');return {background:{url:bg,video_url:'/test-video.webm'},banners:[{url:bg+'#banner1',jump_url:'https://example.com/banner1'},{url:bg+'#banner2',jump_url:'https://example.com/banner2'}],news_tabs:[{tabName:'Уведомления',announcements:Array.from({length:8},(_,i)=>({content:['Обновление игры: основные изменения','Новое событие — подробности и награды','Расписание технических работ'][i%3]+' '+i,jump_url:'https://example.com/news'}))},{tabName:'События',announcements:[]}],sidebars:[{media:'VK',jump_url:'https://example.com/vk',sidebar_labels:[{content:'VK'}]},{media:'Telegram',jump_url:'https://example.com/telegram',sidebar_labels:[{content:'Telegram'}]},{media:'discord',jump_url:'https://example.com/discord',sidebar_labels:[{content:'Discord'}]}]};
case 'check_game_state':if(state.offline)return Promise.reject('Connection refused');return !state.installed?{status:'not_installed',latest_version:'1.1'}:scenario==='update'?{status:'update_available',installed_version:'1.0',latest_version:'1.1'}:{status:'ready',version:settings.installed_version};
case 'check_system_requirements':return {platform:scenario==='windows'?'windows':'linux',has_proton:state.proton,has_ntsync:true,has_gamemode:false,has_mangohud:true,has_gamescope:true,has_vkbasalt:false,has_nvidia:false,hybrid_graphics:true};
case 'get_install_plan':if(state.planError)return Promise.reject('Manifest unavailable');return {download_bytes:48000000000,unpacked_bytes:82000000000,disks:[{path:settings.game_dir,available:scenario==='disk-full'?10000000000:170000000000,required:130000000000}],blocked:scenario==='disk-full'};
case 'plugin:dialog|open':return '/home/player/Games/Existing';
case 'import_existing_game':state.installed=true;settings.game_dir=args.path;settings.installed_version='1.0';return;
case 'start_download':case 'start_update':case 'download_dwproton':case 'verify_game_integrity':{const slot=cmd==='download_dwproton'?'proton':'game';const kind={start_download:'install',start_update:'update',download_dwproton:'proton',verify_game_integrity:'integrity'}[cmd];const task={id:String(Date.now()),slot,kind,status:'running',game_dir:settings.game_dir,progress:{stage:'fetching'},error:null};state.transfers[slot]=task;emit('task://changed',task);return new Promise((resolve,reject)=>state.promises[slot]={resolve,reject});}
case 'stop_transfer':{const old=state.transfers[args.slot];state.transfers[args.slot]={...old,status:args.discard?'idle':'paused'};emit('task://changed',state.transfers[args.slot]);state.promises[args.slot]?.reject('Download cancelled');return;}
case 'launch_game':state.launches=(state.launches||0)+1;return new Promise((resolve,reject)=>state.launch={resolve,reject});
case 'is_game_running':return false;
case 'stop_game':emit('game://exited',{});return;
case 'get_game_sessions':return [];
case 'get_game_version':return {version:'1.1'};
case 'recommended_proton_tag':return 'dwproton-10.0-26';
case 'list_dwproton_releases':return [{tag_name:'dwproton-10.0-26',size:400000000,published_at:'2026-01-01'},{tag_name:'dwproton-11.0',size:410000000,published_at:'2026-01-02'}];
case 'list_installed_protons':return state.proton?[{name:'dwproton-10.0-26',path:settings.proton_dir}]:[];
case 'get_prefix_info':return {exists:true,path:'/home/player/.local/share/llauncher/prefix'};
case 'get_optiscaler_status':return {installed:scenario==='optiscaler',version:scenario==='optiscaler'?'v0.9.4':'',game_dir_missing:!state.installed};
case 'get_mods_status':return {loader_installed:true,loader_configured:true,efmi:true,mod_count:3,reshade_installed:false,game_dir_missing:!state.installed};
case 'read_launch_log':if(state.logError)return Promise.reject('Permission denied');return 'Example log\\nSecond line';
case 'get_debug_info':return 'Example debug info';
case 'plugin:autostart|is_enabled':return state.auto;case 'plugin:autostart|enable':state.auto=true;return;case 'plugin:autostart|disable':state.auto=false;return;
case 'plugin:window|is_focused':return state.visible&&!state.minimized;case 'plugin:window|is_visible':return state.visible;case 'plugin:window|is_minimized':return state.minimized;
case 'plugin:app|version':return '0.3.3';default:return null;
}},{shouldMockEvents:true});
ReactDOM.createRoot(document.getElementById('root')).render(React.createElement(React.StrictMode,null,React.createElement(App)));
`;
