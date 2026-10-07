"""Offline embedding/extraction worker. stdout is reserved for framed MessagePack.

All file paths come from the local Rust broker. No networking code exists here.
Native diagnostics are redirected to stderr before importing LiteRT.
"""
from __future__ import annotations
import argparse
import hashlib
import json
import math
import os
from pathlib import Path
import struct
import subprocess
import sys
import tempfile
import shutil
import time
from typing import Protocol

MAX_FRAME = 64 * 1024 * 1024

class EmbeddingBackend(Protocol):
    def initialize(self, config: dict) -> dict: ...
    def embed(self, items: list[dict]) -> bytes: ...
    def shutdown(self) -> None: ...
    def health_check(self) -> dict: ...

class LiteRTBackend:
    def __init__(self, resources: Path, cache: Path):
        self.resources, self.cache = resources, cache
        self.engine = None
        self.media_engine = None
        self.config = {}
        self.name = 'unloaded'
        self.timings = {}
        self.probes = []

    def initialize(self, config):
        import litert_lm as lm
        if not hasattr(lm, 'EmbeddingEngine'):
            raise RuntimeError('Bundled LiteRT-LM has no EmbeddingEngine API; reinstall a compatible release')
        self.shutdown()
        self.config = config
        if config.get('profile') == 'quiet': config['threads'] = min(config.get('threads', 4), 2)
        elif config.get('profile') == 'fast': config['threads'] = min(config.get('threads', 4), os.cpu_count() or 4)
        (self.cache / 'litert').mkdir(parents=True, exist_ok=True)
        manifest = json.loads((self.resources / 'models/model-manifest.json').read_text())
        model = self.resources / 'models' / manifest['file_name']
        if not model.is_file() or model.stat().st_size != manifest['size']:
            raise RuntimeError('Bundled model missing or incorrect size. Repair the installation.')
        # Cache successful integrity verification keyed by version, size and mtime.
        marker = self.cache / 'model-integrity.json'
        signature = [manifest['sha256'], model.stat().st_size, model.stat().st_mtime_ns]
        verified = json.loads(marker.read_text()) if marker.exists() else None
        if verified != signature:
            with model.open('rb') as f:
                if hashlib.file_digest(f, 'sha256').hexdigest() != manifest['sha256']:
                    raise RuntimeError('Bundled model checksum mismatch. Repair the installation.')
            marker.write_text(json.dumps(signature))
        requested = config.get('accelerator', 'auto')
        candidates = ['gpu', 'npu', 'cpu'] if requested == 'auto' else list(dict.fromkeys([requested, 'cpu']))
        self.probes = []
        best = None
        for candidate in candidates:
            engine = None
            try:
                backend = {'cpu': lambda: lm.Backend.CPU(thread_count=config.get('threads', 4)),
                           'gpu': lambda: lm.Backend.GPU(), 'npu': lambda: lm.Backend.NPU()}[candidate]()
                engine = lm.EmbeddingEngine(str(model), backend=backend, vision_backend=lm.Backend.CPU(),
                    audio_backend=lm.Backend.CPU(thread_count=config.get('threads', 4)),
                    cache_dir=str(self.cache / 'litert') if str(self.cache).isascii() else None,
                    min_input_length=32, max_input_length=512,
                    vision_tokens_per_image=config.get('vision_tokens', 70))
                options = lm.EmbeddingOptions(normalize=False, output_size=768)
                engine.compute_embedding('task: search result | query: warmup', options)
                samples = []
                for _ in range(3):
                    t = time.perf_counter()
                    vec = engine.compute_embedding('task: search result | query: local files', options).embedding
                    normalize(vec, config.get('dimensions', 256))
                    samples.append((time.perf_counter()-t)*1000)
                ms = sum(samples) / len(samples)
                self.probes.append({'backend': candidate, 'available': True, 'query_ms': ms})
                if best is None or ms < best[0]:
                    if best is not None: best[1].close()
                    best = (ms, engine, candidate)
                    engine = None
            except Exception as e:
                self.probes.append({'backend': candidate, 'available': False, 'error': str(e)[:400]})
            finally:
                if engine is not None: engine.close()
        if best is None:
            raise RuntimeError('No backend passed initialization and inference: ' + json.dumps(self.probes))
        self.engine, self.name = best[1], best[2]
        self.model_path = str(model)
        self.timings['query_ms'] = best[0]
        return self.health_check()

    def content(self, item):
        from litert_lm import Content
        mode = item.get('modality', 'text')
        text = item.get('text', '')
        if mode in ('text', 'code'):
            if item.get('query'):
                prefix = 'code retrieval' if mode == 'code' else 'search result'
                return f'task: {prefix} | query: {text}'
            return f'title: {item.get("title", "none")} | text: {text}'
        if mode == 'image': return Content.ImageFile(str(Path(item['path']).resolve()))
        if mode == 'audio': return Content.AudioFile(str(Path(item['path']).resolve()))
        contents = []
        if text: contents.append(Content.Text(text))
        contents.extend(Content.ImageFile(str(Path(p).resolve())) for p in item.get('frames', []))
        if item.get('path'): contents.append(Content.AudioFile(str(Path(item['path']).resolve())))
        if not contents: raise ValueError('Composite input is empty')
        return contents

    def embed(self, items):
        directories=[]
        prepared=[]
        try:
            for item in items:
                if item.get('modality') in ('audio','video') and item.get('start') is not None:
                    item,directory=prepare_segment(item,self.resources,self.cache,self.config)
                    directories.append(directory)
                prepared.append(item)
            return self.embed_prepared(prepared)
        finally:
            for directory in directories:shutil.rmtree(directory,ignore_errors=True)

    def embed_prepared(self, items):
        import litert_lm as lm
        import psutil
        media=any(item.get('modality') in ('audio', 'video', 'composite') for item in items)
        if self.engine is None and not media:self.initialize(self.config)
        if self.engine is None and self.media_engine is None: raise RuntimeError('Embedding model is not ready')
        options = lm.EmbeddingOptions(normalize=False, output_size=768,
            input_overflow_strategy=lm.InputOverflowStrategy.TRUNCATE)
        contents = [self.content(item) for item in items]
        engine = self.engine
        if media:
            if self.media_engine is None:
                if psutil.virtual_memory().available<3*1024**3 and self.engine is not None:
                    self.engine.close();self.engine=None
                self.media_engine = lm.EmbeddingEngine(self.model_path,
                    backend=lm.Backend.CPU(thread_count=self.config.get('threads',4)),
                    vision_backend=lm.Backend.CPU(),audio_backend=lm.Backend.CPU(),
                    min_input_length=32,max_input_length=8192,
                    vision_tokens_per_image=self.config.get('vision_tokens',70))
            engine = self.media_engine
        t = time.perf_counter()
        # This calls the real native batch API when the installed runtime exposes it.
        if len(contents) > 1 and hasattr(engine, 'compute_embedding_batch'):
            results = engine.compute_embedding_batch(contents, options)
        else:
            results = [engine.compute_embedding(c, options) for c in contents]
        self.timings['last_embed_ms'] = (time.perf_counter()-t)*1000
        return b''.join(struct.pack(f'<{self.config["dimensions"]}f', *normalize(r.embedding, self.config['dimensions'])) for r in results)

    def benchmark(self):
        import concurrent.futures
        samples = []
        for count in (1, 4, 8, 16, 32):
            t = time.perf_counter()
            self.embed([{'text': 'A local document about architecture and software.', 'title': 'benchmark.txt'}] * count)
            ms = (time.perf_counter()-t)*1000
            samples.append({'batch': count, 'ms': ms, 'per_second': count*1000/ms})
        # Concurrent calls are tested, not enabled based on a theoretical core count.
        concurrency = []
        for workers in (1, 2, 4):
            t = time.perf_counter()
            with concurrent.futures.ThreadPoolExecutor(max_workers=workers) as pool:
                list(pool.map(lambda _: self.embed([{'text': 'benchmark', 'query': True}]), range(4)))
            concurrency.append({'workers': workers, 'ms': (time.perf_counter()-t)*1000})
        import platform
        manifest=json.loads((self.resources / 'models/model-manifest.json').read_text())
        result = self.health_check() | {'batches': samples, 'concurrency': concurrency,
            'model_sha256':manifest['sha256'],'app_version':'0.1.0','runtime_version':'0.18.0',
            'hardware_fingerprint':hashlib.sha256((platform.processor()+str(os.cpu_count())).encode()).hexdigest(),
            'measured_at':time.time()}
        (self.cache / 'benchmark.json').write_text(json.dumps(result))
        return result

    def shutdown(self):
        if self.media_engine is not None:
            self.media_engine.close()
            self.media_engine = None
        if self.engine is not None:
            self.engine.close()
            self.engine = None

    def health_check(self):
        return {'ready': self.engine is not None or self.media_engine is not None, 'backend': self.name, 'probes': self.probes,
                'dimensions': self.config.get('dimensions', 256), 'timings': self.timings,
                'native_batch': bool(self.engine and hasattr(self.engine, 'compute_embedding_batch')),
                'vision_backend': 'cpu', 'audio_backend': 'cpu'}

def normalize(values, dim):
    if dim not in (128,256,512,768) or len(values) != 768: raise ValueError('Unexpected embedding dimension')
    values = [float(v) for v in values[:dim]]
    if not all(math.isfinite(v) for v in values): raise ValueError('Non-finite embedding')
    norm = math.sqrt(sum(v*v for v in values))
    if norm <= 1e-12: raise ValueError('Zero embedding')
    return [v/norm for v in values]

def run_process(args):
    if Path(args[0]).name.lower()=='ffmpeg.exe':args=[args[0],'-threads','2',*args[1:]]
    result = subprocess.run([str(a) for a in args], capture_output=True, timeout=120,
        creationflags=(subprocess.CREATE_NO_WINDOW|subprocess.BELOW_NORMAL_PRIORITY_CLASS) if os.name == 'nt' else 0)
    if result.returncode: raise RuntimeError('Media decoder failed: ' + result.stderr.decode(errors='replace')[-500:])
    return result.stdout

def extract(path:Path, kind:str, resources:Path, cache:Path, config:dict):
    chunks = []
    jobdir = cache / 'media' / hashlib.sha256(str(path).encode()).hexdigest()[:24]
    jobdir.mkdir(parents=True, exist_ok=True)
    def add(text='', **extra):
        # Rust performs page/sheet-aware chunking after extraction.
        chunks.append({'modality': 'text', 'text': text, **extra})
    if kind == 'pdf':
        import pypdfium2 as pdfium
        with pdfium.PdfDocument(str(path)) as doc:
            for i in range(len(doc)):
                page = doc[i]
                textpage = page.get_textpage()
                text = textpage.get_text_range()
                textpage.close()
                if text.strip(): add(text, page=i+1)
                if config.get('images', True):
                    output = jobdir / f'page-{i+1}.jpg'
                    bitmap = page.render(scale=1.5)
                    bitmap.to_pil().convert('RGB').save(output, quality=85)
                    bitmap.close()
                    chunks.append({'modality': 'image', 'media_path': str(output), 'page': i+1, 'text': ''})
                page.close()
    elif path.suffix.lower() == '.docx':
        from docx import Document
        doc = Document(str(path))
        heading = ''
        for p in doc.paragraphs:
            if p.style.name.startswith('Heading'): heading = p.text
            if p.text.strip(): add(p.text, heading=heading)
        for table in doc.tables:
            add('\n'.join('\t'.join(c.text for c in row.cells) for row in table.rows), heading=heading)
    elif path.suffix.lower() == '.pptx':
        from pptx import Presentation
        for i,slide in enumerate(Presentation(str(path)).slides):
            text = '\n'.join(shape.text for shape in slide.shapes if shape.has_text_frame)
            if slide.has_notes_slide: text += '\n'+slide.notes_slide.notes_text_frame.text
            add(text,page=i+1,heading=f'Slide {i+1}')
    elif path.suffix.lower() == '.xlsx':
        from openpyxl import load_workbook
        workbook = load_workbook(str(path),read_only=True,data_only=True)
        try:
            for sheet in workbook:
                block=[]; start=1
                for index,row in enumerate(sheet.iter_rows(values_only=True),1):
                    if any(v is not None for v in row): block.append('\t'.join(str(v) if v is not None else '' for v in row))
                    if index % 32 == 0:
                        if block: add('\n'.join(block),heading=sheet.title,line_start=start,line_end=index)
                        block=[];start=index+1
                if block: add('\n'.join(block),heading=sheet.title,line_start=start,line_end=index)
        finally: workbook.close()
    elif path.suffix.lower() == '.ods':
        from odf.opendocument import load
        from odf.table import Table,TableRow,TableCell
        from odf import teletype
        doc=load(str(path))
        for table in doc.spreadsheet.getElementsByType(Table):
            rows=[]
            for row in table.getElementsByType(TableRow):
                values=[teletype.extractText(c) for c in row.getElementsByType(TableCell)]
                if any(values): rows.append('\t'.join(values))
            for start in range(0,len(rows),32): add('\n'.join(rows[start:start+32]),heading=table.getAttribute('name'),line_start=start+1,line_end=min(start+32,len(rows)))
    elif kind == 'image':
        from PIL import Image,ImageOps
        with Image.open(path) as image:
            output=jobdir/'image.jpg'
            img=ImageOps.exif_transpose(image).convert('RGB');img.thumbnail((1600,1600));img.save(output,quality=88)
            metadata=' '.join(str(v) for v in image.getexif().values() if isinstance(v,str))
        chunks.append({'modality':'image','text':metadata,'media_path':str(output)})
    elif kind in ('audio','video'):
        ffmpeg=resources/'ffmpeg/bin/ffmpeg.exe';ffprobe=resources/'ffmpeg/bin/ffprobe.exe'
        if not ffmpeg.exists(): raise RuntimeError('Bundled FFmpeg is missing')
        info=json.loads(run_process([ffprobe,'-v','error','-show_format','-show_streams','-of','json',path]))
        duration=float(info['format']['duration'])
        seconds=config.get('audio_seconds',60) if kind=='audio' else config.get('video_seconds',30)
        overlap=5.; start=0.;seq=0
        while start<duration:
            end=min(start+seconds,duration)
            chunks.append({'modality':kind,'text':'','media_path':str(path),'frames':[],'start':start,'end':end})
            if end==duration:break
            start+=seconds-overlap;seq+=1
    else: raise ValueError('Unsupported extractor')
    for chunk in chunks:
        chunk['hash']=hashlib.sha256((json.dumps({k:v for k,v in chunk.items() if k not in ('media_path','frames')},sort_keys=True)).encode()).hexdigest()
    return chunks

def prepare_segment(item,resources,cache,config):
    path=Path(item['path']);base=cache/'segments';base.mkdir(parents=True,exist_ok=True)
    directory=Path(tempfile.mkdtemp(prefix='segment-',dir=base));ffmpeg=resources/'ffmpeg/bin/ffmpeg.exe';ffprobe=resources/'ffmpeg/bin/ffprobe.exe'
    try:
        info=json.loads(run_process([ffprobe,'-v','error','-show_streams','-of','json',path]))
        start=item['start'];duration=item['end']-start;prepared=dict(item);prepared['path']=None
        if any(s['codec_type']=='audio' for s in info['streams']):
            audio=directory/'audio.wav';run_process([ffmpeg,'-v','error','-y','-ss',start,'-i',path,'-t',duration,'-vn','-ac','1','-ar','16000',audio]);prepared['path']=str(audio)
        if item['modality']=='video':
            run_process([ffmpeg,'-v','error','-y','-ss',start,'-i',path,'-t',duration,'-an','-vf',f'fps={config.get("video_fps",1)},scale=640:-2',directory/'frame-%04d.jpg'])
            prepared['frames']=[str(p) for p in sorted(directory.glob('frame-*.jpg'))]
        return prepared,directory
    except BaseException:
        shutil.rmtree(directory,ignore_errors=True);raise

def trim_cache(cache,limit):
    entries=[]
    for name in ('media','segments','previews'):
        directory=cache/name
        if directory.exists():
            for entry in directory.iterdir():
                size=sum(p.stat().st_size for p in entry.rglob('*') if p.is_file()) if entry.is_dir() else entry.stat().st_size
                entries.append((entry.stat().st_mtime,size,entry))
    total=sum(e[1] for e in entries)
    for _,size,entry in sorted(entries):
        if total<=limit*1024*1024:break
        if entry.is_dir():shutil.rmtree(entry,ignore_errors=True)
        else:entry.unlink(missing_ok=True)
        total-=size

def read_frame(stream):
    prefix=stream.read(4)
    if not prefix:return None
    if len(prefix)!=4:raise EOFError('Truncated frame header')
    length=struct.unpack('<I',prefix)[0]
    if length>MAX_FRAME:raise ValueError('Frame too large')
    data=bytearray()
    while len(data)<length:
        block=stream.read(length-len(data))
        if not block:raise EOFError('Truncated frame')
        data.extend(block)
    import msgpack
    return msgpack.unpackb(data,raw=False)

def render_pdf_page(path,page,cache):
    import pypdfium2 as pdfium
    path=Path(path);signature=f'{path}|{path.stat().st_mtime_ns}|{page}'
    folder=cache/'previews';folder.mkdir(parents=True,exist_ok=True)
    output=folder/(hashlib.sha256(signature.encode()).hexdigest()+'.jpg')
    with pdfium.PdfDocument(str(path)) as document:
        if not 1<=page<=len(document):raise ValueError('Invalid PDF page')
        if not output.exists():
            current=document[page-1];bitmap=current.render(scale=min(2,900/max(current.get_width(),1)))
            bitmap.to_pil().convert('RGB').save(output,quality=88);bitmap.close();current.close()
        return {'asset':str(output),'page':page,'count':len(document)}

def write_frame(stream,response):
    import msgpack
    payload=msgpack.packb(response,use_bin_type=True)
    if len(payload)>MAX_FRAME:raise ValueError('Response exceeds limit')
    stream.write(struct.pack('<I',len(payload)));stream.write(payload);stream.flush()

def main():
    # OpenVINO is used only through Core for local device detection/inference.
    # Model optimizer/CLI telemetry integrations are never invoked.
    parser=argparse.ArgumentParser();parser.add_argument('--resources',type=Path,required=True);parser.add_argument('--cache',type=Path,required=True)
    args=parser.parse_args();args.cache.mkdir(parents=True,exist_ok=True)
    # Save the IPC handle, then redirect native stdout writes away from the protocol.
    output=os.fdopen(os.dup(sys.stdout.fileno()),'wb',buffering=0)
    os.dup2(sys.stderr.fileno(),sys.stdout.fileno())
    backend=LiteRTBackend(args.resources,args.cache)
    config={}
    while request:=read_frame(sys.stdin.buffer):
        t=time.perf_counter();rid=request['id'];op=request['operation'];arguments=request.get('arguments',{})
        try:
            binary=b''
            if op=='initialize':config=arguments;trim_cache(args.cache,config.get('cache_mb',512));data=backend.initialize(config)
            elif op=='configure':config=arguments;backend.config=config;data=backend.health_check()
            elif op=='health':data=backend.health_check()
            elif op=='embed':binary=backend.embed(arguments['items']);data={'rows':len(arguments['items']),'dimensions':config['dimensions']}
            elif op=='extract':data={'chunks':extract(Path(arguments['path']),arguments['kind'],args.resources,args.cache,arguments.get('config',config))}
            elif op=='render_pdf_page':data=render_pdf_page(arguments['path'],int(arguments['page']),args.cache);trim_cache(args.cache,config.get('cache_mb',512))
            elif op=='benchmark':data=backend.benchmark()
            elif op=='shutdown':backend.shutdown();data={}
            else:raise ValueError('Unknown operation')
            write_frame(output,{'id':rid,'status':'ok','data':data,'binary':binary,'elapsed_ms':(time.perf_counter()-t)*1000})
        except Exception as e:
            write_frame(output,{'id':rid,'status':'error','error':str(e)[:1000],'data':{},'binary':b'','elapsed_ms':(time.perf_counter()-t)*1000})
    backend.shutdown()

if __name__=='__main__':main()
