#!/usr/bin/env python3
"""Compare grey-conform state with reference at a given block."""
import argparse, os, signal, socket, struct, subprocess, sys, time
from pathlib import Path

KEY_NAMES = {1:"alpha",2:"phi",3:"beta",4:"gamma",5:"psi",6:"eta",
    7:"iota",8:"kappa",9:"lambda",10:"rho",11:"tau",12:"chi",
    13:"pi",14:"omega",15:"xi",16:"theta"}

def send_msg(sock, data):
    sock.sendall(struct.pack('<I', len(data)) + data)

def recv_msg(sock, timeout=60):
    sock.settimeout(timeout)
    hdr = b''
    while len(hdr) < 4:
        chunk = sock.recv(4 - len(hdr))
        if not chunk: return None
        hdr += chunk
    length = struct.unpack('<I', hdr)[0]
    data = b''
    while len(data) < length:
        chunk = sock.recv(min(65536, length - len(data)))
        if not chunk: return None
        data += chunk
    return data

def read_compact(data, pos):
    if pos >= len(data): return 0, pos
    header = data[pos]; pos += 1; length = 0; tmp = header
    while tmp & 0x80: length += 1; tmp = (tmp << 1) & 0xFF
    if length == 8:
        return int.from_bytes(data[pos:pos+8], 'little'), pos+8
    threshold = 0 if length == 0 else 256 - (1 << (8 - length))
    header_value = header - threshold; low = 0
    for i in range(length): low |= data[pos] << (8*i); pos += 1
    return (header_value << (8*length)) | low, pos

def parse_state(data):
    if not data or data[0] != 0x05: return None
    pos = 1; count, pos = read_compact(data, pos); kvs = {}
    for _ in range(count):
        key = bytes(data[pos:pos+31]); pos += 31
        vlen, pos = read_compact(data, pos)
        kvs[key] = bytes(data[pos:pos+vlen]); pos += vlen
    return kvs

def decode_key(key):
    if key[1:] == b'\x00'*30: return KEY_NAMES.get(key[0], f'C({key[0]})')
    if key[0] == 255:
        sid = key[1]|(key[3]<<8)|(key[5]<<16)|(key[7]<<24)
        return f'svc_acct(s={sid})'
    sid = key[0]|(key[2]<<8)|(key[4]<<16)|(key[6]<<24)
    sub = key[1]|(key[3]<<8)|(key[5]<<16)|(key[7]<<24)
    if sub == 0xFFFFFFFF: return f'svc_storage(s={sid})'
    elif sub == 0xFFFFFFFE: return f'svc_preimage(s={sid})'
    else: return f'svc_preimage_info(s={sid},len={sub})'

def decode_svc_account(val):
    if len(val) < 73: return {"raw": val.hex()[:100]}
    p=0; ver=val[p]; p+=1; ch=val[p:p+32].hex(); p+=32
    b=int.from_bytes(val[p:p+8],'little'); p+=8
    g=int.from_bytes(val[p:p+8],'little'); p+=8
    m=int.from_bytes(val[p:p+8],'little'); p+=8
    o=int.from_bytes(val[p:p+8],'little'); p+=8
    f=int.from_bytes(val[p:p+8],'little'); p+=8
    i=int.from_bytes(val[p:p+4],'little'); p+=4
    r=int.from_bytes(val[p:p+4],'little'); p+=4
    a=int.from_bytes(val[p:p+4],'little'); p+=4
    pp=int.from_bytes(val[p:p+4],'little'); p+=4
    return dict(ver=ver, code_hash=ch[:16]+'...', balance=b,
        min_item_gas=g, min_memo_gas=m, bytes_footprint=o,
        deposit_offset=f, items=i, creation_slot=r, last_accum=a, parent_svc=pp)

def find_ref():
    for c in ["/tmp/conformance-releases/tiny/linux/x86_64/jam_conformance_target","/tmp/jam_conformance_target"]:
        if os.path.exists(c): return c

def get_state(binary, sock_path, args, trace_dir, block_num, name):
    if os.path.exists(sock_path): os.unlink(sock_path)
    env = os.environ.copy(); env['JAM_CONSTANTS']='tiny'; env['RUST_LOG']='warn'
    proc = subprocess.Popen([binary]+args, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL, env=env)
    for _ in range(50):
        if os.path.exists(sock_path): break
        time.sleep(0.1)
    else: print(f"  {name}: socket timeout"); proc.kill(); return None
    try:
        s = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM); s.settimeout(30); s.connect(sock_path)
        files = sorted(trace_dir.glob('*_fuzzer_*.bin'))
        n_msgs = block_num + 2
        for i in range(min(n_msgs, len(files))):
            send_msg(s, files[i].read_bytes())
            resp = recv_msg(s)
            if resp is None: s.close(); return None
        if n_msgs < len(files):
            send_msg(s, bytes([0x04]) + files[n_msgs].read_bytes()[1:33])
            state_resp = recv_msg(s, timeout=120)
            kvs = parse_state(state_resp); s.close(); return kvs
        s.close(); return None
    finally:
        proc.send_signal(signal.SIGTERM)
        try: proc.wait(timeout=5)
        except: proc.kill(); proc.wait()
        if os.path.exists(sock_path): os.unlink(sock_path)

def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("block", type=int)
    ap.add_argument("--trace", default="res/conformance/fuzz-proto/examples/0.7.2/no_forks")
    ap.add_argument("--our-bin", default="target/release/grey-conform")
    ap.add_argument("--ref-bin", default=None)
    args = ap.parse_args()
    td = Path(args.trace); td = Path(os.getcwd())/td if not td.is_absolute() else td
    ob = args.our_bin; ob = str(Path(os.getcwd())/ob) if not os.path.isabs(ob) else ob
    rb = args.ref_bin or find_ref()
    if not rb: print("ref binary not found"); sys.exit(1)
    pid = os.getpid()
    print(f"Comparing state after block {args.block}...")
    ref_kvs = get_state(rb, f'/tmp/cmp_ref_{pid}.sock', ['--socket',f'/tmp/cmp_ref_{pid}.sock','--exit-on-disconnect'], td, args.block, 'REF')
    our_kvs = get_state(ob, f'/tmp/cmp_ours_{pid}.sock', [f'/tmp/cmp_ours_{pid}.sock'], td, args.block, 'OURS')
    if not our_kvs or not ref_kvs: print("FAILED"); sys.exit(1)
    all_keys = sorted(set(list(our_kvs.keys())+list(ref_kvs.keys())))
    print(f"\nKeys: ours={len(our_kvs)} ref={len(ref_kvs)}")
    mm=oo=ro=0
    for key in all_keys:
        name = decode_key(key)
        if key in our_kvs and key in ref_kvs:
            if our_kvs[key] != ref_kvs[key]:
                ov,rv = our_kvs[key],ref_kvs[key]; mm+=1
                print(f'\nMISMATCH {name}\n  key: {key.hex()}\n  ours: {len(ov)}b  ref: {len(rv)}b')
                if 'svc_acct' in name:
                    od,rd = decode_svc_account(ov),decode_svc_account(rv)
                    for k in od:
                        if od[k]!=rd[k]: print(f'    {k}: ours={od[k]} ref={rd[k]}')
                elif len(ov)<=200 and len(rv)<=200:
                    print(f'  ours: {ov.hex()}\n  ref:  {rv.hex()}')
                else:
                    ml=min(len(ov),len(rv))
                    for i in range(ml):
                        if ov[i]!=rv[i]:
                            s,e=max(0,i-16),min(i+48,ml)
                            print(f'  diff@byte {i}:\n    ours[{s}:{e}]={ov[s:e].hex()}\n    ref [{s}:{e}]={rv[s:e].hex()}')
                            break
                    if len(ov)!=len(rv): print(f'  len diff: {len(ov)} vs {len(rv)}')
        elif key in our_kvs: oo+=1; print(f'\nOURS_ONLY {name}\n  {our_kvs[key].hex()[:200]}')
        else: ro+=1; print(f'\nREF_ONLY {name}\n  {ref_kvs[key].hex()[:200]}')
    print(f'\n{"="*60}\nSummary: {len(all_keys)} keys, {mm} mismatch, {oo} ours-only, {ro} ref-only')
    if mm==0 and oo==0 and ro==0: print("PERFECT MATCH!")
    sys.exit(0 if mm==0 and oo==0 and ro==0 else 1)

if __name__ == '__main__': main()
