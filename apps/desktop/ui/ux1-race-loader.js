// Isolate module/bootstrap failures so Create and Settings can explain recovery.
window.NT2_RACE_LOAD={status:'loading',error:''};
import('./ux1-race.js').then(()=>{window.NT2_RACE_LOAD.status='ready';}).catch(error=>{
 delete window.NT2_RACE_UI;
 window.NT2_RACE_LOAD={status:'error',error:'Race editor could not load. Restart the app. If this continues, use Capture Logs and report the error.'};
 console.error('Race editor bootstrap failed',error);
 const target=document.getElementById('error');if(target)target.textContent=window.NT2_RACE_LOAD.error;
});
