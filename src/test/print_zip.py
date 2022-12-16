#import zip
import zipfile
import os
import re
from pathlib import Path

def get_workspace_files(path):
    
    res = []
    for root, dirs, fnames in os.walk(path):
        for fname in fnames:
            res.append( str( (Path(root) / fname).relative_to(path) ) )
    return set(res)
    
def get_zip_contents():    
    zip = zipfile.ZipFile('/modules/CMR/working/cmr_comparison.zip')

    files = []
    for file in zip.namelist():
        file_info = zip.getinfo(file)
        if not file_info.is_dir():
            files.append(file)

    print(f"Found {len(files)} files in zip")
    return set(files )
    
def filter_by_regexes(regex_list, files):

    res = []
    for f in files:
        for r in regex_list:
            if r.match(f):
                res.append(f)
                break
                
    return set(res )
    
def main():
    zip_files = get_zip_contents()
    
    workspace_list = get_workspace_files('/modules/CMR/working')
    
    print(f"Found {len(workspace_list)} files in working directory")
    
    re_flags = re.VERBOSE | re.IGNORECASE
    regex_list = [
        #re.compile(".*(?:building_csv).*", re_flags),
        re.compile(".*(xlsx|csv)", re_flags),
        re.compile("rust_comparison_data.*(?:fgb|tif|csv)", re_flags),
        re.compile("[^/]*(tif|toml|jpg)", re_flags),
        re.compile(".*v_polygons.*fgb", re_flags),
        re.compile("qa/.*", re_flags),
        
    ]
    filtered = filter_by_regexes(regex_list, workspace_list)
    
    print(f"Found {len(filtered)} filtered files")
    
    missed_files = list(zip_files - filtered)
    
    extra_files = list(filtered - zip_files)
    
    print(f"Missed files: {len(missed_files)} {missed_files[0:10]}")
    
    print(f"Extra files: {len(extra_files)} {extra_files[0:10]}")
    
    
    
if __name__ == "__main__":
    main()
#print(files)