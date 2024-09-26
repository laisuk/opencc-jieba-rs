from python.opencc_jieba_rs import OpenCC

config = "s2t"
# input_text = "白日依山尽，黄河入海流"
# input_text = "潦水尽而寒潭清，烟光凝而暮山紫。俨骖𬴂于上路，访风景于崇阿"
input_text = "「100人参加了这次活动」"
input_text2 = "數大了似乎按照著一種自然律，自然的會有一種特別的排列，一種特別的節奏，一種特殊的式樣，激動我們審美的本能，激發我們審美的情緒。"
opencc = OpenCC()
converted = opencc.convert(input_text)
text_code = opencc.zho_check(input_text)
cut_str = opencc.jieba_cut(input_text, True)
cut_str_join = opencc.jieba_cut_and_join(input_text2, True, "/ ")
str_list = ['白日', '依山', '尽', '，', '黄河', '入海流']
join_str_list = opencc.jieba_join_str(str_list, "; ")
print(converted)
print(text_code)
print(input_text[-2:])
print(cut_str)
print(cut_str_join)
print(join_str_list)
