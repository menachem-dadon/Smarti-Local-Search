import {cleanup,render,screen} from '@testing-library/react';
import {afterEach,describe,expect,it} from 'vitest';
import {IndexEstimate,duration} from './IndexEstimate';
import {type Status} from '../../app/api';
import {en} from '../../i18n/en';
const t=(k:keyof typeof en)=>en[k];
afterEach(cleanup);
describe('index time estimates',()=>{
 it('does not invent a discovery duration before the count is known',()=>{
  render(<IndexEstimate status={{running:true,discovery_counting:true,pending:0} as Status} t={t} language="en"/>);
  expect(screen.getByText(en.estimating)).toBeInTheDocument();expect(screen.getByText(en.afterDiscovery)).toBeInTheDocument();expect(screen.queryByText(en.complete)).not.toBeInTheDocument();
 });
 it('shows measured durations and marks initial estimates',()=>{
  render(<IndexEstimate status={{running:true,discovery_complete:true,discovery_total:1000,discovery_processed:1000,pending:100,index_eta_seconds:3600,index_eta_provisional:true} as Status} t={t} language="en"/>);
  expect(screen.getByText('1 hour')).toBeInTheDocument();expect(screen.getByText(en.estimateProvisional)).toBeInTheDocument();expect(screen.getByText('1,000 / 1,000')).toBeInTheDocument();
 });
 it('hides stale durations while paused or stopped',()=>{
  render(<IndexEstimate status={{running:false,paused:false,pending:100,discovery_complete:false,index_eta_seconds:3600,discovery_eta_seconds:60} as Status} t={t} language="en"/>);
  expect(screen.getAllByText(en.estimatePaused)).toHaveLength(2);expect(screen.queryByText('1 hour')).not.toBeInTheDocument();
 });
 it('formats long durations in both languages',()=>{expect(duration(172800,'en')).toBe('2 days');expect(duration(3600,'he')).toContain('שעה')});
});
