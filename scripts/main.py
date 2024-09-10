import matplotlib.pyplot as plt
from matplotlib.backends.backend_pdf import PdfPages
from matplotlib.table import Table
from PIL import Image
import pandas as pd
import sys
import os

CSVDIR = 'csv_out'
IMGDIR = 'img_out'
PDFDIR = 'pdf_out'

def add_financials_table(fdf, ax):
    ax.axis('tight')
    ax.axis('off')
    ftable = Table(ax, bbox=[0, 0, 1, 1])
    for (j, label) in enumerate(fdf.columns):
        ftable.add_cell(0, j, width=0.1, height=0.05, text=label, loc='center', facecolor='lightgrey')
    for i, row in enumerate(fdf.itertuples(), start=1):
        for j, value in enumerate(row[1:], start=0):
            ftable.add_cell(i, j, width=0.1, height=0.05, text=value, loc='center')
    ax.add_table(ftable)

def generate_pdf(pdf_name, ticker, dt_str):
    with PdfPages(pdf_name) as pdf:
        for csv_file in os.listdir(CSVDIR):
            csv_path = os.path.join(CSVDIR, csv_file)
            csv_type = csv_file.split('_')[1]
            if ticker in csv_file and dt_str in csv_file and csv_type == 'fv':
                try:
                    fdf = pd.read_csv(csv_path)
                    plt.figure(figsize=[12, 8], dpi=100)
                    ax = plt.subplot(111)
                    add_financials_table(fdf, ax)
                    pdf.savefig(bbox_inches='tight')
                    plt.close()
                except Exception as e:
                    print(f'\ngenerate_pdf :: ERROR -> A problem occured while processing financial statement data file {csv_file}:\n\n{e}\n\ngenerate_pdf :: continuing pdf generation without financial statement data table\n\n')
                continue
            else:
                print(f'\ngenerate_pdf :: Skipping csv file {csv_file}')
        print(f'\ngenerate_pdf :: Added financial statement data from {csv_file} to pdf')
        for img_file in os.listdir(IMGDIR):
            if ticker in img_file and dt_str in img_file:
                img_path = os.path.join(IMGDIR, img_file)
                img = Image.open(img_path)
                fig, ax = plt.subplots(figsize=[12, 8], dpi=100)
                ax.axis('off')
                ax.imshow(img)
                pdf.savefig(fig, bbox_inches='tight')
                plt.close(fig)
                print(f'\ngenerate_pdf :: Added {img_path} to pdf')
        print(f'\ngenerate_pdf :: Successfully created pdf report as {pdf_name}')

if __name__ == '__main__':
    if len(sys.argv) != 3:
        print(f'\n__main__ :: ERROR -> Wrong number of arguments passed to scripts/main.py; {len(sys.argv)} arguments found')
        sys.exit()
    ticker = sys.argv[1]
    datetime_str = sys.argv[2]
    pdf_name = PDFDIR + '/' + ticker + '_' + datetime_str + '.pdf' 
    generate_pdf(pdf_name, ticker, datetime_str)