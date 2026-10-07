import io,math,struct,sys,unittest,tempfile
from pathlib import Path
sys.path.insert(0,str(Path(__file__).resolve().parents[1]/'src'))
from host import normalize,read_frame,write_frame,extract

class HostTests(unittest.TestCase):
    def test_mrl_normalization(self):
        vector=normalize([1.]*768,256)
        self.assertEqual(len(vector),256)
        self.assertAlmostEqual(sum(x*x for x in vector),1)
        with self.assertRaises(ValueError):normalize([float('nan')]*768,256)
        with self.assertRaises(ValueError):normalize([0.]*768,256)
        with self.assertRaises(ValueError):normalize([1.]*256,256)
    def test_frame_binary(self):
        stream=io.BytesIO();value={'id':42,'binary':struct.pack('<2f',1.,2.)};write_frame(stream,value);stream.seek(0);self.assertEqual(read_frame(stream),value)
    def test_frame_limit(self):
        with self.assertRaises(ValueError):read_frame(io.BytesIO(struct.pack('<I',100_000_000)))
        with self.assertRaises(EOFError):read_frame(io.BytesIO(b'\x01'))
    def test_document_extraction(self):
        root=Path(__file__).resolve().parents[2]
        with tempfile.TemporaryDirectory() as cache:
            for name in ('meeting.docx','architecture.pptx','performance.xlsx','notes.ods','search.pdf','beach.png','tone.wav','beach.mp4'):
                path=root/'tests/fixtures/corpus'/name
                kind='image' if name.endswith('png') else 'audio' if name.endswith('wav') else 'video' if name.endswith('mp4') else 'pdf' if name.endswith('pdf') else 'document'
                chunks=extract(path,kind,root/'resources',Path(cache),{'images':True})
                self.assertGreater(len(chunks),0,name)
                if kind=='pdf':self.assertEqual(chunks[0]['page'],1)
                if kind in ('audio','video'):self.assertEqual(chunks[0]['start'],0.)

if __name__=='__main__':unittest.main()
